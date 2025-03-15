use crate::gate::tcp::session::{TcpSessionActor, TcpSessionConnectedMessage, TcpSessionMessage};
use crate::gate::{HandleFn, LogicMessage};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::{ActorStopReason, PanicError};
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::{messages, Actor};
use std::ops::ControlFlow;
use tokio::net::TcpStream;

pub struct TcpClientActor {
    addr: String,
    client_ref: Option<ActorRef<TcpSessionActor>>,
    handle_fn: HandleFn,
}

impl TcpClientActor {
    pub fn new(addr: String, handle_fn: HandleFn) -> Self {
        Self {
            handle_fn,
            addr,
            client_ref: None,
        }
    }
}

impl Actor for TcpClientActor {
    type Mailbox = UnboundedMailbox<TcpClientActor>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let client = TcpStream::connect(self.addr.as_str()).await?;
        tracing::info!("Tcp:connected connection: {}", self.addr);
        let connection_actor_ref =
            kameo::actor::spawn_link(&actor_ref, TcpSessionActor::new(self.handle_fn)).await;
        connection_actor_ref
            .tell(TcpSessionConnectedMessage { stream: client })
            .await?;
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
            client.tell(TcpSessionMessage::Send(message)).await?;
        }
        Ok(())
    }
}
