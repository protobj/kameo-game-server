use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use network::tcp::message::LogicMessage;
use network::tcp::session::{Reader, SessionMessage, Writer};
use network::tcp::stream::{NetworkStream, ReaderHalf, WriterHalf};

pub trait MessageHandler: Send + 'static {
    type Session: Actor;
    fn message_handle(
        &self,
        actor_ref: ActorRef<Self::Session>,
        logic_message: LogicMessage,
    ) -> impl Future<Output = ()> + Send;
}

pub struct Session<H>
where
    H: MessageHandler<Session = Self>,
{
    network_stream: Option<NetworkStream>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader<Self>>>,
    message_handler: H,
}

impl<H> Session<H>
where
    H: MessageHandler<Session = Self>,
{
    pub fn new(network_stream: NetworkStream, message_handler: H) -> Self {
        Self {
            network_stream: Some(network_stream),
            writer: None,
            reader: None,
            message_handler,
        }
    }
}
impl<T: MessageHandler<Session = Self>> Actor for Session<T> {
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

impl<H: MessageHandler<Session = Self>> Message<SessionMessage> for Session<H> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            SessionMessage::Write(message) => {
                if let Some(writer) = self.writer.as_mut() {
                    writer
                        .tell(network::tcp::session::Write(message))
                        .await
                        .unwrap();
                }
            }
            SessionMessage::Read(message) => {
                self.message_handler
                    .message_handle(ctx.actor_ref(), message)
                    .await;
            }
        }
    }
}
