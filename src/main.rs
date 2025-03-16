use bytes::Bytes;
use kameo::actor::ActorRef;
use kameo::remote::dial_opts::DialOpts;
use kameo::remote::ActorSwarm;
use servers::gate::{LogicMessage, NetMessage};
use std::{pin::Pin, time::Duration};
use kameo::actor;
use tracing_subscriber::EnvFilter;
use net::tcp::{client::NetClientActor, listener::NetServerActor};

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
    let actor = NetServerActor::new();

    let actor_ref = actor::spawn(actor);



    tokio::time::sleep(Duration::from_secs(1));

    actor::spawn(NetClientActor::new());


    tokio::signal::ctrl_c().await?;

    Ok(())
}
