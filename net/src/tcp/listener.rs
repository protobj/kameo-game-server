use crate::tcp::LogicMessage;
use crate::tcp::session::{NetSessionActor, TcpSessionMessage};
use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::error::Infallible;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::Message;
use message_io::events::EventReceiver;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self, NodeTask, StoredNodeEvent};
use message_io::node::{NodeEvent, NodeHandler};
use std::collections::HashMap;

pub struct NetServerActor {
    session_actors: HashMap<Endpoint, ActorRef<NetSessionActor>>,
    task: NodeTask,
    receiver: EventReceiver<StoredNodeEvent<()>>,
    handler: NodeHandler<()>,
    session: Option<ActorRef<NetSessionActor>>,
}
impl NetServerActor {
    pub fn new() -> Self {
        let (handler, listener) = node::split::<()>();

        let addr = ("127.0.0.1", 2323);
        match handler.network().listen(Transport::FramedTcp, addr) {
            Ok((_id, real_addr)) => println!(
                "Server running at {} by {}",
                real_addr,
                Transport::FramedTcp
            ),
            Err(_) => {
                println!(
                    "Can not listening at {:?} by {}",
                    addr,
                    Transport::FramedTcp
                );
            }
        }

        let (task, receiver) = listener.enqueue();

        Self {
            session_actors: HashMap::new(),
            task,
            receiver,
            handler,
            session: None,
        }
    }
}
impl Actor for NetServerActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = Infallible;
    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref.tell(Start).await.expect("eeererer");
        Ok(())
    }
}

struct Start;

impl Message<Start> for NetServerActor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: Start,
        ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let receiver = &mut self.receiver;
        match receiver.receive() {
            StoredNodeEvent::Network(stored_net_event) => match stored_net_event {
                node::StoredNetEvent::Connected(_, _) => (),
                node::StoredNetEvent::Accepted(endpoint, _resource_id) => {
                    println!("NetServerActor Accepted ");
                    let session_actor_ref = kameo::actor::spawn_link(
                        &ctx.actor_ref(),
                        NetSessionActor::new(endpoint, self.handler.clone()),
                    )
                    .await;
                    self.session_actors
                        .insert(endpoint, session_actor_ref.clone());
                    self.session = Some(session_actor_ref);
                }
                node::StoredNetEvent::Message(endpoint, items) => {
                    let new = items.clone();
                    let message: LogicMessage = LogicMessage::from(items);
                    let session_ref: ActorRef<NetSessionActor> =
                        self.session_actors[&endpoint].clone();
                    session_ref.tell(TcpSessionMessage::Read(message)).await?;

                    let session =  self.session.take().unwrap();
                    
                    session.tell(TcpSessionMessage::Read(LogicMessage::from(new))).await?;
                }
                node::StoredNetEvent::Disconnected(endpoint) => {
                    self.session_actors.remove(&endpoint);
                }
            },
            StoredNodeEvent::Signal(_) => (),
        }
        Ok(ctx.actor_ref().tell(msg).await?)
    }
}
