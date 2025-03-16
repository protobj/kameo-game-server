use kameo::{error::Infallible, mailbox::unbounded::UnboundedMailbox, message::Message, Actor};
use message_io::{
    events::EventReceiver,
    network::Transport,
    node::{self, NodeHandler, NodeTask, StoredNetEvent, StoredNodeEvent},
};

use super::LogicMessage;

pub struct NetClientActor {
    task: NodeTask,
    receiver: EventReceiver<StoredNodeEvent<()>>,
    handler: NodeHandler<()>,
}

impl NetClientActor {
    pub fn new() -> NetClientActor {
        let (handler, listener) = node::split::<()>();

        let remote_addr = "127.0.0.1:2323";
        let (server_id, local_addr) = handler
            .network()
            .connect(Transport::FramedTcp, remote_addr)
            .unwrap();

        let (task, receiver) = listener.enqueue();

        Self {
            task,
            receiver,
            handler,
        }
    }
}
impl Actor for NetClientActor {
    type Mailbox = UnboundedMailbox<Self>;

    type Error = Infallible;

    async fn on_start(
        &mut self,
        actor_ref: kameo::actor::ActorRef<Self>,
    ) -> Result<(), Self::Error> {
        actor_ref.tell(Start).await.expect("client error");
        Ok(())

    }
}

struct Start;

impl Message<Start> for NetClientActor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: Start,
        ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let receiver = &mut self.receiver;
        match receiver.receive() {
            StoredNodeEvent::Network(stored_net_event) => match stored_net_event {
                StoredNetEvent::Connected(endpoint, _) => {
                    let msg = LogicMessage {
                        ix: 1,
                        cmd: 2,
                        bytes: "hello_world".into(),
                    };

                    self.handler
                        .network()
                        .send(endpoint, LogicMessage::to_vec(msg).as_ref());
                }
                StoredNetEvent::Accepted(endpoint, resource_id) => (),
                StoredNetEvent::Message(endpoint, items) => {
                    let msg: LogicMessage = items.into();

                    println!(
                        "msg: ix:{} cmd:{} bytes:{}",
                        msg.ix,
                        msg.cmd,
                        String::from_utf8_lossy(msg.bytes.as_ref())
                    )
                }
                StoredNetEvent::Disconnected(endpoint) => (),
            },
            StoredNodeEvent::Signal(_) => (),
        };

      ctx
            .actor_ref()
            .tell(msg)
            .await?;
      Ok(())
    }
}
