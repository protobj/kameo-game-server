use crate::network::{LogicMessage, MessageHandler, NetMessage};
use bytes::{BufMut, Bytes, BytesMut};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::Mailbox;
use kameo::mailbox::bounded::BoundedMailbox;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

// 定义一个 TCP Actor 来处理单个连接
pub struct TcpConnectionActor {
    pub(crate) message_handler: Box<dyn MessageHandler>,
    pub(self) writer: Option<ActorRef<TcpConnectionActorWriter>>,
    pub(self) reader: Option<ActorRef<TcpConnectionActorReader>>,
}

impl Actor for TcpConnectionActor {
    type Mailbox = UnboundedMailbox<TcpConnectionActor>;
    type Error = anyhow::Error;
    async fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::error!("TCP connection actor is stopping: {}", reason);
        self.message_handler
            .handle(&actor_ref.upgrade().unwrap(), NetMessage::Close)
            .await?;
        Ok(())
    }
}

impl TcpConnectionActor {
    pub fn new(message_handler: Box<dyn MessageHandler>) -> Self {
        Self {
            message_handler,
            writer: None,
            reader: None,
        }
    }
}

pub enum TcpConnectionMessage {
    Read(LogicMessage),
    Send(LogicMessage),
    Close,
}

impl Message<TcpConnectionMessage> for TcpConnectionActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TcpConnectionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            TcpConnectionMessage::Read(message) => {
                self.message_handler
                    .handle(&ctx.actor_ref(), NetMessage::Read(message))
                    .await;
            }
            TcpConnectionMessage::Send(message) => {
                if let Some(writer) = self.writer.as_mut() {
                    if let Err(e) = writer
                        .tell(TcpConnectionActorWriteMessage {
                            net_message: message,
                        })
                        .await
                    {
                        tracing::error!("Failed to send message: {:?}", e);
                        ctx.actor_ref().kill();
                    }
                }
            }
            TcpConnectionMessage::Close => {
                ctx.actor_ref().kill();
            }
        }
    }
}
pub(crate) struct TcpConnectionConnectedMessage {
    pub(crate) stream: TcpStream,
}

impl Message<TcpConnectionConnectedMessage> for TcpConnectionActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TcpConnectionConnectedMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (read_half, write_half) = msg.stream.into_split();
        self.writer = Some(
            kameo::actor::spawn_link(&ctx.actor_ref(), TcpConnectionActorWriter { write_half })
                .await,
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
    }
}

struct TcpConnectionActorReader {
    tcp_connection_ref: ActorRef<TcpConnectionActor>,
    read_half: OwnedReadHalf,
}
impl Actor for TcpConnectionActorReader {
    type Mailbox = UnboundedMailbox<TcpConnectionActorReader>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        tracing::info!(
            "TCP connection actor is started id:{} start reading",
            actor_ref.id()
        );
        Ok(actor_ref.tell(TcpConnectionActorReaderWaitForMsg).await?)
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

struct TcpConnectionActorReaderWaitForMsg;

impl Message<TcpConnectionActorReaderWaitForMsg> for TcpConnectionActorReader {
    type Reply = ();

    async fn handle(
        &mut self,
        wait_for_msg: TcpConnectionActorReaderWaitForMsg,
        ctx: &mut Context< Self, Self::Reply>,
    ) -> Self::Reply {
        let read_half = &mut self.read_half;
        let len = read_half.read_u32().await;
        match len {
            Ok(len) => match read_n_bytes(read_half, len as usize).await {
                Ok(mut read_bytes) => {
                    tracing::info!("Read {} bytes", read_bytes.len());
                    let prefix = read_bytes.drain(..6).collect::<Vec<_>>();
                    let ix = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);
                    let cmd = u16::from_le_bytes([prefix[4], prefix[5]]);
                    let bytes = Bytes::from(read_bytes);
                    let message = LogicMessage { ix, cmd, bytes };
                    self.tcp_connection_ref
                        .tell(TcpConnectionMessage::Read(message))
                        .await
                        .expect("error handling connection");
                }
                Err(err) => {
                    tracing::error!("read err:{:?}", err);
                    let actor_ref = ctx.actor_ref();
                    if actor_ref.is_alive() {
                        actor_ref.kill();
                        actor_ref.wait_for_stop().await;
                    }
                }
            },
            Err(err) => {
                tracing::error!("Tcp connection actor reader fail:{}", err);
                let actor_ref = ctx.actor_ref();
                if actor_ref.is_alive() {
                    actor_ref.kill();
                    actor_ref.wait_for_stop().await;
                }
            }
        }
        let actor_ref = ctx.actor_ref();
        if actor_ref.is_alive() {
            actor_ref
                .tell(wait_for_msg)
                .await
                .expect("error handling connection");
        }
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

struct TcpConnectionActorWriter {
    write_half: OwnedWriteHalf,
}
impl Actor for TcpConnectionActorWriter {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

}
struct TcpConnectionActorWriteMessage {
    net_message: LogicMessage,
}

impl Message<TcpConnectionActorWriteMessage> for TcpConnectionActorWriter {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TcpConnectionActorWriteMessage,
        ctx: &mut Context< Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.write_half.writable().await {
            tracing::error!("write wait err:{:?}", e);
        }
        let len = 4 + 2 + msg.net_message.bytes.len();
        let mut write_bytes = BytesMut::with_capacity(len);
        write_bytes.put_u32(len as u32);
        write_bytes.put_u32_le(msg.net_message.ix);
        write_bytes.put_u16_le(msg.net_message.cmd);
        write_bytes.put(msg.net_message.bytes);
        if let Err(write_err) = self.write_half.write_all(&write_bytes).await {
            tracing::error!("write err:{}", write_err);
            ctx.actor_ref().kill();
        }
        if let Err(e) = self.write_half.flush().await {
            tracing::error!("write flush err:{}", e);
        }
    }
}
