use crate::{
    MessageHandler, NetworkStreamInfo, Package, PackageType, SessionMessage, bytes_to_usize,
};
use bytes::{Buf, BytesMut};
use fastwebsockets::{
    Frame, OpCode, Payload, Role, WebSocket, WebSocketError, WebSocketRead, WebSocketWrite,
};
use futures_util::{SinkExt, StreamExt};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::message::{Context, Message};
use std::net::SocketAddr;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
pub struct WsSession<H>
where
    H: MessageHandler<Actor = Self>,
{
    network_stream_info: NetworkStreamInfo,
    network_stream: Option<WebSocket<TcpStream>>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader<Self>>>,
    message_handler: H,
}

impl<H> WsSession<H>
where
    H: MessageHandler<Actor = Self>,
{
    pub async fn new(
        network_stream_info: NetworkStreamInfo,
        network_stream: TcpStream,
        message_handler: H,
    ) -> anyhow::Result<Self> {
        let mut ws_socket = WebSocket::after_handshake(network_stream, Role::Server);
        ws_socket.set_auto_close(true);
        ws_socket.set_auto_pong(true);
        Ok(Self {
            network_stream_info,
            network_stream: Some(ws_socket),
            writer: None,
            reader: None,
            message_handler,
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.network_stream_info.local_addr
    }
    pub fn peer_addr(&self) -> SocketAddr {
        self.network_stream_info.peer_addr
    }
}
impl<T: MessageHandler<Actor = Self>> Actor for WsSession<T> {
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let network_steam = self.network_stream.take().unwrap();
        let (reader_half, writer_half) = network_steam.split(tokio::io::split);
        let writer = kameo::actor::spawn_link(&actor_ref, Writer::new(writer_half)).await;
        let reader =
            kameo::actor::spawn_link(&actor_ref, Reader::new(actor_ref.clone(), reader_half)).await;
        self.writer = Some(writer);
        self.reader = Some(reader);
        Ok(())
    }
}

impl<H: MessageHandler<Actor = Self>> Message<SessionMessage> for WsSession<H> {
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
    reader: Option<WebSocketRead<ReadHalf<TcpStream>>>,
    join_handle: Option<JoinHandle<()>>,
}
impl<T: Actor + Message<SessionMessage>> Reader<T> {
    pub fn new(session_ref: ActorRef<T>, reader: WebSocketRead<ReadHalf<TcpStream>>) -> Self {
        Self {
            session_ref,
            reader: Some(reader),
            join_handle: None,
        }
    }
}

impl<T: Actor + Message<SessionMessage>> Actor for Reader<T> {
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let mut read_half = self.reader.take().unwrap();
        let session_ref = self.session_ref.clone();
        let handle = tokio::task::spawn(async move {
            loop {
                let frame = read_half
                    .read_frame::<_, WebSocketError>(&mut move |_| async {
                        unreachable!();
                    })
                    .await;
                match frame {
                    Ok(mut frame) => match frame.opcode {
                        OpCode::Binary => {
                            let x = frame.payload.to_mut();
                            let mut bytes_mut = BytesMut::with_capacity(x.len());
                            frame.payload.to_mut().copy_from_slice(bytes_mut.as_ref());
                            let package_type = PackageType::from_u8(bytes_mut.get_u8());
                            let r#type = if let Some(option) = package_type {
                                option
                            } else {
                                return;
                            };
                            let package = match r#type {
                                PackageType::Handshake
                                | PackageType::HandshakeAck
                                | PackageType::Heartbeat
                                | PackageType::Kick => Package::new_control(r#type),
                                PackageType::Request
                                | PackageType::Response
                                | PackageType::ResponseError
                                | PackageType::Notify
                                | PackageType::Push => {
                                    Package::new_data(r#type, bytes_mut.as_ref())
                                }
                            };
                            session_ref
                                .tell(SessionMessage::Read(package))
                                .await
                                .expect("tell to session_ref error");
                        }
                        OpCode::Close => {
                            session_ref.stop_gracefully().await;
                            session_ref.wait_for_stop().await;
                        }
                        _ => {}
                    },
                    Err(_) => {}
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
    writer: WebSocketWrite<WriteHalf<TcpStream>>,
}
impl Writer {
    pub fn new(writer: WebSocketWrite<WriteHalf<TcpStream>>) -> Self {
        Self { writer }
    }
}
impl Actor for Writer {
    type Error = anyhow::Error;
}
struct Write(pub Package);

impl Message<Write> for Writer {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, msg: Write, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let message = msg.0;
        let bytes = Package::to_bytes(message);
        let stream = &mut self.writer;
        stream
            .write_frame(Frame::binary(Payload::Borrowed(bytes.as_ref())))
            .await?;
        Ok(())
    }
}
