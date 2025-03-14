use anyhow::Context;
use bytes::Bytes;
use common::logging::init_logging;
use common::network::tcp::client::{ClientSend, Connect, TcpClientActor};
use common::network::tcp::session::TcpConnectionActor;
use common::network::{LogicMessage, MessageHandler, MessageHandlerFactory, NetMessage};
use kameo::actor::ActorRef;
use kameo::remote::ActorSwarm;
use kameo::remote::dial_opts::DialOpts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(["debug".parse()?].to_vec());

    let actor_swarm =
        ActorSwarm::bootstrap().map_err(|e| anyhow::anyhow!("failed to bootstrap:{}", e))?;
    actor_swarm
        .dial(
            DialOpts::unknown_peer_id()
                .address("/ip4/0.0.0.0/udp/8020/quic-v1".parse()?)
                .build(),
        )
        .await?;

    struct Handler;
    struct HandlerFactory;
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
        }
    }

    let tcp_actor = TcpClientActor::new("0.0.0.0:4567".to_string(), Box::new(HandlerFactory));
    let tcp_actor_ref = kameo::spawn(tcp_actor);
    loop {
        if tcp_actor_ref.ask(Connect).await? {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    for i in 0..10 {
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
