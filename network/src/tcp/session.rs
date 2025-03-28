use crate::{
    MessageHandler, NetworkStreamInfo, Package, PackageType, SessionMessage, bytes_to_usize,
};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::message::{Context, Message};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::task::JoinHandle;

pub struct TcpSession<H>
where
    H: MessageHandler<Actor = Self>,
{
    network_stream_info: NetworkStreamInfo,
    network_stream: Option<TcpStream>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader<Self>>>,
    message_handler: H,
}

impl<H> TcpSession<H>
where
    H: MessageHandler<Actor = Self>,
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
impl<T: MessageHandler<Actor = Self>> Actor for TcpSession<T> {
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

impl<H: MessageHandler<Actor = Self>> Message<SessionMessage> for TcpSession<H> {
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
    type Error = io::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let mut read_half = self.reader.take().unwrap();
        let session_ref = self.session_ref.clone();
        let handle = tokio::task::spawn(async move {
            loop {
                let result = parse(&mut read_half).await;
                match result {
                    Ok(package) => {
                        session_ref
                            .tell(SessionMessage::Read(package))
                            .await
                            .expect("TODO: panic message");
                    }
                    Err(e) => {}
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

async fn parse(reader: &mut BufReader<OwnedReadHalf>) -> anyhow::Result<Package> {
    let r#type = reader.read_u8().await?;
    let option = PackageType::from_u8(r#type);
    let r#type = if let Some(option) = option {
        option
    } else {
        anyhow::bail!("invalid option type: {}", r#type);
    };
    match r#type {
        PackageType::Handshake
        | PackageType::HandshakeAck
        | PackageType::Heartbeat
        | PackageType::Kick => Ok(Package::new_control(r#type)),
        PackageType::Request
        | PackageType::Response
        | PackageType::ResponseError
        | PackageType::Notify
        | PackageType::Push => {
            let len_buf = read_n_bytes(reader, 3).await?;
            let length = bytes_to_usize(len_buf);
            let contents = read_n_bytes(reader, length).await?;
            tracing::trace!("Payload length message ({length}) received");
            Ok(Package::new_data(r#type, contents.as_slice()))
        }
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
    type Error = anyhow::Error;
}
struct Write(pub Package);

impl Message<Write> for Writer {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, msg: Write, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let message = msg.0;

        let bytes = Package::to_bytes(message);

        let stream = &mut self.writer;
        if let Err(write_err) = stream.write_all(&bytes).await {
            tracing::warn!("Error writing to the stream '{}'", write_err);
            let actor_ref = ctx.actor_ref();
            actor_ref.stop_gracefully().await?;
            actor_ref.wait_for_stop().await;
            return Ok(());
        }
        stream.flush().await?;
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
