use std::pin::Pin;
use anyhow::Ok;
use kameo::actor::ActorRef;
use kameo::remote::ActorSwarm;
use tracing_subscriber::EnvFilter;
use servers::gate::NetMessage;
// use servers::gate::tcp::listener::TcpServerActor;
// use servers::gate::tcp::session::{TcpSessionActor, TcpSessionMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info".parse::<EnvFilter>()?)
        .without_time()
        .with_target(false)
        .init();
    Ok(())
    // let _ = ActorSwarm::bootstrap()?
    //     .listen_on("/ip4/0.0.0.0/udp/8020/quic-v1".parse()?)
    //     .await?;
    // fn handle(
    //     actor_ref: ActorRef<TcpSessionActor>,
    //     net_message: NetMessage,
    // ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    //     Box::pin(async move {
    //         match net_message {
    //             NetMessage::Read(message) => {
    //                 actor_ref
    //                     .tell(TcpSessionMessage::Send(message))
    //                     .await
    //                     .unwrap();
    //             }
    //             NetMessage::Close => {
    //                 tracing::info!("Received close message");
    //             }
    //         }
    //     })
    // }
    // let tcp_actor = TcpServerActor::new("0.0.0.0:4567".to_string(), handle);
    // let _ = kameo::spawn(tcp_actor);
    // tokio::signal::ctrl_c()
    //     .await
    //     .expect("failed to listen for event");
    // Ok(())
}
