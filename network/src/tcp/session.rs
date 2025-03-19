use crate::{LogicMessage, MessageHandler, NetworkStreamInfo, SessionMessage};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::task::JoinHandle;

pub struct TcpSession<H>
where
    H: MessageHandler<Session = Self>,
{
    network_stream_info: NetworkStreamInfo,
    network_stream: Option<TcpStream>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader<Self>>>,
    message_handler: H,
}

impl<H> TcpSession<H>
where
    H: MessageHandler<Session = Self>,
{
    pub fn new(
        network_stream_info: NetworkStreamInfo,
        network_stream: TcpStream,
        message_handler: H,
    ) -> Self {
        Self {
            network_stream_info,
            network_stream: Some(network_stream),
            writer: None,
            reader: None,
            message_handler,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.network_stream_info.local_addr
    }
    pub fn peer_addr(&self) -> SocketAddr {
        self.network_stream_info.peer_addr
    }
}
impl<T: MessageHandler<Session = Self>> Actor for TcpSession<T> {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let network_steam = self.network_stream.take().unwrap();
        let (reader_half, writer_half) = network_steam.into_split();
        let writer = kameo::actor::spawn_link(&actor_ref, Writer::new(writer_half)).await;
        let reader =
            kameo::actor::spawn_link(&actor_ref, Reader::new(actor_ref.clone(), reader_half)).await;
        self.writer = Some(writer);
        self.reader = Some(reader);
        Ok(())
    }
}

impl<H: MessageHandler<Session = Self>> Message<SessionMessage> for TcpSession<H> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            SessionMessage::Write(message) => {
                if let Some(writer) = self.writer.as_mut() {
                    writer.tell(Write(message)).await.unwrap();
                }
            }
            SessionMessage::Read(message) => {
                self.message_handler
                    .message_read(ctx.actor_ref(), message)
                    .await;
            }
        }
    }
}

struct Reader<T>
where
    T: Actor + Message<SessionMessage>,
{
    session_ref: ActorRef<T>,
    reader: Option<BufReader<OwnedReadHalf>>,
    join_handle: Option<JoinHandle<()>>,
}
impl<T: Actor + Message<SessionMessage>> Reader<T> {
    pub fn new(session_ref: ActorRef<T>, reader: OwnedReadHalf) -> Self {
        Self {
            session_ref,
            reader: Some(BufReader::new(reader)),
            join_handle: None,
        }
    }
}

impl<T: Actor + Message<SessionMessage>> Actor for Reader<T> {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let mut read_half = self.reader.take().unwrap();
        let session_ref = self.session_ref.clone();
        let handle = tokio::task::spawn(async move {
            loop {
                match read_half.read_u32().await {
                    Ok(length) => {
                        tracing::trace!("Payload length message ({length}) received");
                        match read_n_bytes(&mut read_half, length as usize).await {
                            Ok(buf) => {
                                tracing::trace!("Payload of length({}) received", buf.len());
                                // NOTE: Our implementation writes 2 messages when sending something over the wire, the first
                                // is exactly 8 bytes which constitute the length of the payload message (u64 in big endian format),
                                // followed by the payload. This tells our TCP reader how much data to read off the wire
                                session_ref
                                    .tell(SessionMessage::Read(LogicMessage::from(buf.as_slice())))
                                    .await
                                    .expect("tell to session_ref error");
                            }
                            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                                //关闭连接
                                actor_ref.kill();
                                actor_ref.wait_for_stop().await;
                            }
                            Err(_other_err) => {
                                tracing::trace!("Read:Error ({_other_err:?}) on stream")
                            }
                        }
                    }
                    Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                        tracing::trace!("Error (EOF) on stream");
                        // EOF, close the stream by dropping the stream
                        //关闭连接
                        actor_ref.kill();
                        actor_ref.wait_for_stop().await;
                    }
                    Err(_other_err) => {
                        tracing::trace!("WaitFor:Error ({_other_err:?}) on stream")
                        // some other TCP error, more handling necessary
                    }
                }
            }
        });
        self.join_handle = Some(handle);
        Ok(())
    }
    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        //终止读取
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

struct Writer {
    writer: BufWriter<OwnedWriteHalf>,
}
impl Writer {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        Self {
            writer: BufWriter::new(writer),
        }
    }
}
impl Actor for Writer {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}
struct Write(pub LogicMessage);

impl Message<Write> for Writer {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, msg: Write, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let message = msg.0;

        let bytes = LogicMessage::to_bytes(&message);
        let stream = &mut self.writer;
        if let Err(write_err) = stream.write_u32(bytes.len() as u32).await {
            tracing::warn!("Error writing to the stream '{}'", write_err);
        } else {
            tracing::trace!("Wrote length, writing payload (len={})", bytes.len());
            // now send the frame
            if let Err(write_err) = stream.write_all(&bytes).await {
                tracing::warn!("Error writing to the stream '{}'", write_err);
                let actor_ref = ctx.actor_ref();
                actor_ref.kill();
                actor_ref.wait_for_stop().await;
            }
            // flush the stream
            stream.flush().await?;
        }
        Ok(())
    }
}
async fn read_n_bytes(
    read_half: &mut BufReader<OwnedReadHalf>,
    len: usize,
) -> Result<Vec<u8>, tokio::io::Error> {
    let mut buf = vec![0u8; len];
    let mut c_len = 0;
    while c_len < len {
        let n = read_half.read(buf.as_mut_slice()).await?;
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
