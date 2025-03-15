use crate::gate::{HandleFn, LogicMessage, NetMessage, handle};
use bytes::{BufMut, Bytes, BytesMut};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::{ActorStopReason, PanicError};
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use std::ops::ControlFlow;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

// 定义一个 TCP Actor 来处理单个连接
pub struct TcpSessionActor {
    pub handle_fn: HandleFn,
    pub(self) writer: Option<ActorRef<TcpSessionActorWriter>>,
    pub(self) reader: Option<ActorRef<TcpConnectionActorReader>>,
}

#[derive(Clone, Error, Debug)]
pub enum TcpSessionError {
    #[error("connection error")]
    Tell,
}

impl Actor for TcpSessionActor {
    type Mailbox = UnboundedMailbox<TcpSessionActor>;
    type Error = TcpSessionError;
    async fn on_panic(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        err: PanicError,
    ) -> Result<ControlFlow<ActorStopReason>, Self::Error> {
        tracing::error!("TCP connection  on_panic actor is stopping: {}", err);
        Ok(ControlFlow::Break(ActorStopReason::Panicked(err)))
    }
    async fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::error!("TCP connection actor is stopping: {}", reason);
        Ok(())
    }
}

impl TcpSessionActor {
    pub fn new(handle_fn: HandleFn) -> Self {
        Self {
            handle_fn,
            writer: None,
            reader: None,
        }
    }
}

pub enum TcpSessionMessage {
    Read(LogicMessage),
    Send(LogicMessage),
    Close,
}

impl Message<TcpSessionMessage> for TcpSessionActor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: TcpSessionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            TcpSessionMessage::Read(message) => {
                let handle_fn = self.handle_fn;
                handle_fn(ctx.actor_ref(), NetMessage::Read(message)).await;
                Ok(())
            }
            TcpSessionMessage::Send(message) => {
                if let Some(writer) = self.writer.as_mut() {
                    writer
                        .tell(TcpConnectionActorWriteMessage {
                            net_message: message,
                        })
                        .await?
                }
                anyhow::bail!("TCP connection actor is closed")
            }
            TcpSessionMessage::Close => {
                ctx.actor_ref().kill();
                Ok(())
            }
        }
    }
}
pub(crate) struct TcpSessionConnectedMessage {
    pub(crate) stream: TcpStream,
}

impl Message<TcpSessionConnectedMessage> for TcpSessionActor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: TcpSessionConnectedMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (read_half, write_half) = msg.stream.into_split();
        self.writer = Some(
            kameo::actor::spawn_link(&ctx.actor_ref(), TcpSessionActorWriter { write_half }).await,
        );

        self.reader = Some(
            kameo::actor::spawn_link(
                &ctx.actor_ref(),
                TcpConnectionActorReader {
                    tcp_connection_ref: ctx.actor_ref(),
                    read_half,
                },
            )
            .await,
        );
        Ok(())
    }
}

struct TcpConnectionActorReader {
    tcp_connection_ref: ActorRef<TcpSessionActor>,
    read_half: OwnedReadHalf,
}
impl Actor for TcpConnectionActorReader {
    type Mailbox = UnboundedMailbox<TcpConnectionActorReader>;
    type Error = TcpSessionError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        tracing::info!(
            "TCP connection actor is started id:{} start reading",
            actor_ref.id()
        );
        actor_ref
            .tell(TcpSessionActorReaderWaitForMsg)
            .await
            .map_err(|_| TcpSessionError::Tell)?;
        Ok(())
    }
    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::info!("Tcp connection actor stopping: {:?}", reason);
        Ok(())
    }
}

struct TcpSessionActorReaderWaitForMsg;

impl Message<TcpSessionActorReaderWaitForMsg> for TcpConnectionActorReader {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        wait_for_msg: TcpSessionActorReaderWaitForMsg,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let read_half = &mut self.read_half;
        let len = read_half.read_u32().await? as usize;
        let mut read_bytes = read_n_bytes(read_half, len).await?;

        tracing::info!("Read {} bytes", read_bytes.len());

        let prefix = read_bytes.drain(..6).collect::<Vec<_>>();
        let ix = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);
        let cmd = u16::from_le_bytes([prefix[4], prefix[5]]);
        let bytes = Bytes::from(read_bytes);
        let message = LogicMessage { ix, cmd, bytes };

        self.tcp_connection_ref
            .tell(TcpSessionMessage::Read(message))
            .await?;

        Ok(ctx.actor_ref().tell(wait_for_msg).await?)
    }
}

async fn read_n_bytes(reader: &mut OwnedReadHalf, len: usize) -> Result<Vec<u8>, tokio::io::Error> {
    let mut buf = vec![0u8; len];
    let mut c_len = 0;
    reader.readable().await?;

    while c_len < len {
        let n = reader.read(&mut buf[c_len..]).await?;
        if n == 0 {
            // EOF
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::UnexpectedEof,
                "EOF",
            ));
        }
        c_len += n;
    }
    Ok(buf)
}

struct TcpSessionActorWriter {
    write_half: OwnedWriteHalf,
}
impl Actor for TcpSessionActorWriter {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = TcpSessionError;
}
struct TcpConnectionActorWriteMessage {
    net_message: LogicMessage,
}

impl Message<TcpConnectionActorWriteMessage> for TcpSessionActorWriter {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: TcpConnectionActorWriteMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.write_half.writable().await {
            tracing::trace!("write wait err:{:?}", e);
        }
        let len = 4 + 2 + msg.net_message.bytes.len();
        let mut write_bytes = BytesMut::with_capacity(len);
        write_bytes.put_u32(len as u32);
        write_bytes.put_u32_le(msg.net_message.ix);
        write_bytes.put_u16_le(msg.net_message.cmd);
        write_bytes.put(msg.net_message.bytes);

        self.write_half.write_all(&write_bytes).await?;
        self.write_half.flush().await?;
        Ok(())
    }
}
