use common::logging::init_logging;
use common::network::tcp::listener::TcpServerActor;
use common::network::tcp::session::{TcpConnectionActor, TcpConnectionMessage};
use common::network::{MessageHandler, MessageHandlerFactory, NetMessage};
use kameo::actor::ActorRef;
use kameo::remote::ActorSwarm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(["debug".parse()?].to_vec());
    let actor_swarm = ActorSwarm::bootstrap()
        .map_err(|e| anyhow::anyhow!("failed to bootstrap:{}", e))?
        .listen_on("/ip4/0.0.0.0/udp/8020/quic-v1".parse()?);

    struct Handler;
    struct HandlerFactory;
    #[async_trait::async_trait]
    impl MessageHandlerFactory for HandlerFactory {
        fn create_handler(&self) -> Box<dyn MessageHandler + std::marker::Send + Sync> {
            Box::new(Handler)
        }
    }

    #[async_trait::async_trait]
    impl MessageHandler for Handler {
        async fn handle(&self, actor_ref: &ActorRef<TcpConnectionActor>, message: NetMessage) {
            match message {
                NetMessage::Read(message) => {
                    actor_ref
                        .tell(TcpConnectionMessage::Send(message))
                        .await
                        .unwrap();
                }
                NetMessage::Close => {
                    tracing::info!("Received close message");
                }
            }
        }
    }

    let tcp_actor = TcpServerActor::new("0.0.0.0:4567".to_string(), Box::new(HandlerFactory));
    let tcp_actor_ref = kameo::spawn(tcp_actor);
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
    Ok(())
}
