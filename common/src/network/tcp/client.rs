use crate::network::tcp::session::{
    TcpConnectionActor, TcpConnectionConnectedMessage, TcpConnectionMessage,
};
use crate::network::{LogicMessage, MessageHandlerFactory};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::{ActorStopReason, PanicError};
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::{Actor, messages};
use std::ops::ControlFlow;
use tokio::net::TcpStream;

pub struct TcpClientActor {
    addr: String,
    client: Option<TcpStream>,
    message_handler_factory: Box<dyn MessageHandlerFactory>,
    client_ref: Option<ActorRef<TcpConnectionActor>>,
}

impl TcpClientActor {
    pub fn new(addr: String, message_handler_factory: Box<dyn MessageHandlerFactory>) -> Self {
        Self {
            addr,
            client: None,
            message_handler_factory,
            client_ref: None,
        }
    }
}
#[messages]
impl TcpClientActor {
    #[message]
    pub async fn connect(&mut self) -> bool {
        match self.client_ref {
            None => false,
            Some(_) => true,
        }
    }
    #[message]
    pub async fn client_send(&mut self, message: LogicMessage) -> anyhow::Result<()> {
        if let Some(client) = self.client_ref.as_ref() {
            client.tell(TcpConnectionMessage::Send(message)).await?;
        }
        Ok(())
    }
}
impl Actor for TcpClientActor {
    type Mailbox = UnboundedMailbox<TcpClientActor>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let client = TcpStream::connect(self.addr.as_str()).await?;
        tracing::info!("Tcp:connected connection: {}", self.addr);
        let connection_actor_ref = kameo::actor::spawn_link(
            &actor_ref,
            TcpConnectionActor::new(self.message_handler_factory.create_handler()),
        )
        .await;
        connection_actor_ref
            .tell(TcpConnectionConnectedMessage { stream: client })
            .await
            .expect("error handling connection");
        self.client_ref = Some(connection_actor_ref);
        Ok(())
    }

    async fn on_panic(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        err: PanicError,
    ) -> Result<ControlFlow<ActorStopReason>, Self::Error> {
        tracing::info!("tcp:connected panic: {}", err);
        Ok(ControlFlow::Break(ActorStopReason::Panicked(err)))
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::info!("tcp:connected panic: {:?}", reason);
        drop(self.client_ref.take());
        Ok(())
    }
}
