use crate::gate::tcp::session::Session;
use crate::gate::tcp::stream::{IncomingEncryptionMode, NetworkPort, NetworkStream};
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct Listener {
    port: NetworkPort,
    encryption: IncomingEncryptionMode,
    join_handle: Option<JoinHandle<()>>,
}

impl Listener {
    pub fn new(port: NetworkPort, encryption: IncomingEncryptionMode) -> Self {
        Self {
            port,
            encryption,
            join_handle: None,
        }
    }
}

impl Actor for Listener {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        let encryption = self.encryption.clone();
        let port = self.port.clone();
        let join_handle = tokio::spawn(async move {
            let addr = format!("127.0.0.1:{}", port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(err) => {
                    panic!("Unable to bind tcp listener: {}", err);
                }
            };
            tracing::trace!("Listening on {}", addr);
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let local = stream.local_addr().unwrap();
                        let network_stream = match &encryption {
                            IncomingEncryptionMode::Raw => Some(NetworkStream::Raw {
                                peer_addr: addr,
                                local_addr: local,
                                stream,
                            }),
                            IncomingEncryptionMode::Tls(acceptor) => {
                                match acceptor.accept(stream).await {
                                    Ok(enc_stream) => Some(NetworkStream::TlsServer {
                                        peer_addr: addr,
                                        local_addr: local,
                                        stream: enc_stream,
                                    }),
                                    Err(some_err) => {
                                        tracing::warn!(
                                            "Error establishing secure socket: {some_err}"
                                        );
                                        None
                                    }
                                }
                            }
                        };
                        if let Some(network_stream) = network_stream {
                            tracing::info!("TCP Session opened for {addr}");
                            let server_actor_ref = actor_ref.clone();
                            tokio::spawn(async move {
                                let session = Session::new(network_stream);
                                let _ = kameo::actor::spawn_link(&server_actor_ref, session).await;
                            });
                        }
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
