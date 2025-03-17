use crate::gate::tcp::message::LogicMessage;
use crate::gate::tcp::stream::{NetworkStream, ReaderHalf, WriterHalf};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use kameo::Actor;
use std::io::ErrorKind;
use tokio::task::JoinHandle;

pub struct Session {
    network_stream: Option<NetworkStream>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader>>,
}
impl Session {
    pub fn new(network_stream: NetworkStream) -> Self {
        Self {
            network_stream: Some(network_stream),
            writer: None,
            reader: None,
        }
    }
}
impl Actor for Session {
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
pub enum SessionMessage {
    Write(LogicMessage),
    Read(LogicMessage),
}
impl Message<SessionMessage> for Session {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: SessionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            SessionMessage::Write(message) => {
                if let Some(writer) = self.writer.as_mut() {
                    writer.tell(Write(message)).await?;
                }
            }
            SessionMessage::Read(message) => {
                //TODO logic
                ctx.actor_ref().tell(SessionMessage::Write(message)).await?
            }
        }
        Ok(())
    }
}

pub struct Reader {
    session_ref: ActorRef<Session>,
    reader: Option<ReaderHalf>,
    join_handle: Option<JoinHandle<()>>,
}
impl Reader {
    pub fn new(session_ref: ActorRef<Session>, reader: ReaderHalf) -> Self {
        Self {
            session_ref,
            reader: Some(reader),
            join_handle: None,
        }
    }
}

impl Actor for Reader {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let mut read_half = self.reader.take().unwrap();
        // actor_ref.tell(ReaderMessage::WaitFor).await?;
        let session_ref = self.session_ref.clone();
        let handle = tokio::task::spawn(async move {
            loop {
                match read_half.read_u32().await {
                    Ok(length) => {
                        tracing::trace!("Payload length message ({length}) received");
                        match read_half.read_n_bytes(length as usize).await {
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
    writer: WriterHalf,
}
impl Writer {
    pub fn new(writer: WriterHalf) -> Self {
        Self { writer }
    }
}
impl Actor for Writer {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}
struct Write(LogicMessage);

impl Message<Write> for Writer {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, msg: Write, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let message = msg.0;

        let bytes = LogicMessage::to_bytes(&message);
        let stream = &mut self.writer;
        if let WriterHalf::Regular(w) = stream {
            w.writable().await?;
        }
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
