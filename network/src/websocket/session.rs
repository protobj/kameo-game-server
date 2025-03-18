use crate::tcp::message::LogicMessage;
use crate::tcp::session::SessionMessage;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub struct Reader<T>
where
    T: Actor + Message<SessionMessage>,
{
    session_ref: ActorRef<T>,
    reader: Option<SplitStream<WebSocketStream<TcpStream>>>,
    join_handle: Option<JoinHandle<()>>,
}
impl<T: Actor + Message<SessionMessage>> Reader<T> {
    pub fn new(session_ref: ActorRef<T>, reader: SplitStream<WebSocketStream<TcpStream>>) -> Self {
        Self {
            session_ref,
            reader: Some(reader),
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
                match read_half.next().await {
                    None => {}
                    Some(message) => match message {
                        Ok(msg) => match msg {
                            WsMessage::Binary(bytes) => {
                                let message = LogicMessage::from(bytes.as_ref());
                                session_ref
                                    .tell(SessionMessage::Read(message))
                                    .await
                                    .expect("tell to session_ref error");
                            }
                            WsMessage::Close(_) => {}
                            _ => {}
                        },
                        Err(_) => {}
                    },
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

pub struct Writer {
    writer: SplitSink<WebSocketStream<TcpStream>, WsMessage>,
}
impl Writer {
    pub fn new(writer: SplitSink<WebSocketStream<TcpStream>, WsMessage>) -> Self {
        Self { writer }
    }
}
impl Actor for Writer {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}
pub struct Write(pub LogicMessage);

impl Message<Write> for Writer {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, msg: Write, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let message = msg.0;
        let bytes = LogicMessage::to_bytes(&message);
        let stream = &mut self.writer;
        stream.send(WsMessage::Binary(bytes)).await?;
        Ok(())
    }
}
