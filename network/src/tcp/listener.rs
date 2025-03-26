use crate::{NetworkPort, NetworkStreamInfo, SessionMessage};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::Message;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

pub struct Listener<T>
where
    T: Actor + Message<SessionMessage>,
{
    port: NetworkPort,
    join_handle: Option<JoinHandle<()>>,
    connection_handler: Arc<
        dyn Fn(
                NetworkStreamInfo,
                TcpStream,
            )
                -> Pin<Box<dyn Future<Output = Result<ActorRef<T>, anyhow::Error>> + Send>>
            + Send
            + Sync,
    >,
}

impl<T: Actor + Message<SessionMessage>> Listener<T> {
    pub fn new<F, Fut>(port: NetworkPort, connection_handler: F) -> Self
    where
        F: Fn(NetworkStreamInfo, TcpStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ActorRef<T>, anyhow::Error>> + Send + 'static,
    {
        Self {
            port,
            join_handle: None,
            connection_handler: Arc::new(move |network_stream_info, stream| {
                Box::pin(connection_handler(network_stream_info, stream))
            }),
        }
    }
}

impl<T: Actor + Message<SessionMessage>> Actor for Listener<T> {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let port = self.port.clone();
        let out_connection_handler = self.connection_handler.clone();
        let join_handle = tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(err) => {
                    panic!("Unable to bind tcp listener: {}", err);
                }
            };
            tracing::info!("Listening on {}", addr);
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let local = stream.local_addr().unwrap();
                        let network_stream_info = NetworkStreamInfo {
                            peer_addr: addr,
                            local_addr: local,
                        };
                        tracing::info!("Session opened for {addr}");
                        let server_actor_ref = actor_ref.clone();
                        let connection_handler = out_connection_handler.clone();
                        tokio::spawn(async move {
                            match (connection_handler)(network_stream_info, stream).await {
                                Ok(actor_ref) => server_actor_ref.link(&actor_ref).await,
                                Err(err) => {
                                    tracing::warn!("Connection handler failed session: {err}");
                                }
                            };
                        });
                    }
                    Err(socket_accept_error) => {
                        tracing::warn!("Error accepting socket {socket_accept_error}");
                    }
                }
            }
        });
        self.join_handle = Some(join_handle);
        Ok(())
    }

    async fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}
