use crate::tcp::LogicMessage;
use anyhow::{Ok, anyhow};
use kameo::error::Infallible;
use kameo::Actor;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use message_io::network::{Endpoint, SendStatus};
use message_io::node::NodeHandler;

pub struct NetSessionActor {
    endpoint: Endpoint,
    handler: NodeHandler<()>,
}

impl NetSessionActor {
    pub(crate) fn new(endpoint: Endpoint, handler: NodeHandler<()>) -> Self {

        println!("NetSessionActor new");
        Self { endpoint, handler }
    }
}

impl Message<TcpSessionMessage> for NetSessionActor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: TcpSessionMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            TcpSessionMessage::Read(logic_message) => {
                ctx.actor_ref()
                    .tell(TcpSessionMessage::Send(logic_message))
                    .await?;
            }
            TcpSessionMessage::Send(logic_message) => {
                let send_status = self
                    .handler
                    .network()
                    .send(self.endpoint, LogicMessage::to_vec(logic_message).as_ref());
                if send_status != SendStatus::Sent {
                    anyhow::bail!(anyhow!("{:?}", send_status));
                }
            }
            TcpSessionMessage::Close => {
                ctx.actor_ref().kill();
    
            }
        }
        Ok(())
    }
}

impl Actor for NetSessionActor {
    type Mailbox = UnboundedMailbox<NetSessionActor>;
    type Error = Infallible;

 
}

pub enum TcpSessionMessage {
    Read(LogicMessage),
    Send(LogicMessage),
    Close,
}
