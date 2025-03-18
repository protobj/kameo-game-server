use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use network::tcp::session::{Reader, SessionMessage, Writer};
use network::tcp::stream::{NetworkStream, ReaderHalf, WriterHalf};

pub struct Session {
    network_stream: Option<NetworkStream>,
    writer: Option<ActorRef<Writer>>,
    reader: Option<ActorRef<Reader<Self>>>,
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
                    writer.tell(network::tcp::session::Write(message)).await?;
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
