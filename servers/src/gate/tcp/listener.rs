use crate::gate::tcp::session::{TcpSessionActor, TcpSessionConnectedMessage};
use crate::gate::HandleFn;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use kameo::Actor;
use tokio::net::TcpListener;

// 定义一个主服务器 Actor 来监听 TCP 连接
pub struct TcpServerActor {
    addr: String,
    listener: Option<TcpListener>,
    handle_fn: HandleFn,
}

impl TcpServerActor {
    pub fn new(addr: String, handle_fn: HandleFn) -> Self {
        Self {
            handle_fn,
            addr,
            listener: None,
        }
    }
}

impl Actor for TcpServerActor {
    type Mailbox = UnboundedMailbox<TcpServerActor>;

    type Error = anyhow::Error;
    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let tcp_listener = TcpListener::bind(self.addr.as_str()).await?;
        self.listener = Some(tcp_listener);
        tracing::info!("tcp server actor started on {}", self.addr);
        actor_ref.tell(TcpServerListenMessage).await?;
        Ok(())
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::trace!("Tcp server stopping: {:?}", reason);
        drop(self.listener.take());
        Ok(())
    }
}

struct TcpServerListenMessage;
impl Message<TcpServerListenMessage> for TcpServerActor {
    type Reply = ();

    async fn handle(
        &mut self,
        tcp_server_listen_message: TcpServerListenMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(listener) = &mut self.listener {
            //loop accept
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("Tcp:Accepted connection from: {}", addr);
                    let actor_ref = kameo::actor::spawn_link(
                        &ctx.actor_ref(),
                        TcpSessionActor::new(self.handle_fn),
                    )
                    .await;
                    actor_ref
                        .tell(TcpSessionConnectedMessage { stream })
                        .await
                        .expect("error handling connection");
                }
                Err(e) => {
                    tracing::error!("Tcp:Failed to accept connection:{}", e)
                }
            };
            ctx.actor_ref()
                .tell(tcp_server_listen_message)
                .await
                .expect("error handling connection");
        }
    }
}
