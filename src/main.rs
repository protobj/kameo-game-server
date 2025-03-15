use bytes::Bytes;
use kameo::actor::ActorRef;
use kameo::remote::dial_opts::DialOpts;
use kameo::remote::ActorSwarm;
use servers::gate::tcp::client::{ClientSend, Connect, TcpClientActor};
use servers::gate::tcp::session::TcpSessionActor;
use servers::gate::{LogicMessage, NetMessage};
use std::pin::Pin;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info".parse::<EnvFilter>()?)
        .without_time()
        .with_target(false)
        .init();

    let actor_swarm =
        ActorSwarm::bootstrap().map_err(|e| anyhow::anyhow!("failed to bootstrap:{}", e))?;
    actor_swarm
        .dial(
            DialOpts::unknown_peer_id()
                .address("/ip4/127.0.0.1/udp/8020/quic-v1".parse()?)
                .build(),
        )
        .await?;
    fn handle(
        actor_ref: ActorRef<TcpSessionActor>,
        net_message: NetMessage,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match net_message {
            NetMessage::Read(message) => {
                println!(
                    "recv ix:{} cmd:{} bytes:{}",
                    message.ix,
                    message.cmd,
                    String::from_utf8_lossy(&message.bytes)
                );
            }
            NetMessage::Close => {
                actor_ref.kill();
            }
        }
        Box::pin(async move {})
    }

    let tcp_actor = TcpClientActor::new("0.0.0.0:4567".to_string(), handle);
    let tcp_actor_ref = kameo::spawn(tcp_actor);

    loop {
        if tcp_actor_ref.ask(Connect).await? {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    for i in 0..1 {
        let logic_message = LogicMessage {
            ix: 1,
            cmd: 1,
            bytes: Bytes::from("hello world"),
        };
        tcp_actor_ref
            .tell(ClientSend {
                message: logic_message,
            })
            .await
            .expect("error sending logic message");
    }

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}
