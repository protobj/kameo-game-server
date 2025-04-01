use crate::center::{CenterActor, CenterMessage};
use common::config::{ServerRole, ServerRoleId};
use kameo::actor::RemoteActorRef;
use kameo::prelude::ActorSwarm;
use kameo::remote::dial_opts::DialOpts;
use std::ops::Deref;
use std::time::Duration;
use tokio::sync::watch::Receiver;

pub enum Signal {
    None,
    Stop,
}
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    async fn init(&mut self, mut signal_rx: Receiver<Signal>) -> anyhow::Result<()> {
        tracing::info!("starting node :{}", self.server_role_id());

        self.start().await?;
        loop {
            let changed = signal_rx.changed().await;
            match changed {
                Ok(_) => {
                    let signal = signal_rx.borrow();
                    match signal.deref() {
                        Signal::Stop => {
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::error!("{} signal recv err :{}", self.server_role_id(), e);
                }
            }
        }
        tracing::info!("stopping node :{}", self.server_role_id());
        self.stop().await?;
        tracing::info!("stop node :{}", self.server_role_id());
        Ok(())
    }

    async fn start(&mut self) -> anyhow::Result<()>;

    async fn stop(&mut self) -> anyhow::Result<()>;

    fn server_role_id(&self) -> ServerRoleId;

    async fn start_actor_swarm(
        &self,
        self_address: String,
        other_addresses: Vec<String>,
    ) -> anyhow::Result<()> {
        //启动集群
        let role_id = self.server_role_id();
        let actor_swarm = ActorSwarm::bootstrap()
            .expect(format!("actor_swarm bootstrap failed :{}", role_id).as_str());
        let listener_id = actor_swarm.listen_on(self_address.parse()?).await?;
        tracing::info!(
            "ActorSwarm[{}] listening addr:{} listener_id:{}",
            role_id,
            self_address,
            listener_id
        );
        //连接其他地址
        for other_addr in other_addresses {
            if other_addr == self_address {
                continue;
            }
            actor_swarm
                .dial(
                    DialOpts::unknown_peer_id()
                        .address(other_addr.parse()?)
                        .build(),
                )
                .await?;
        }

        Ok(())
    }
    async fn connect_center(&self) -> anyhow::Result<RemoteActorRef<CenterActor>> {
        let actor_swarm = ActorSwarm::get().unwrap();
        let server_role_id = self.server_role_id();
        if server_role_id.0 == ServerRole::Center {
            return Err(anyhow::anyhow!("cant connect to self"));
        }
        //连接Center

        let actor_ref = RemoteActorRef::<CenterActor>::lookup(&ServerRole::Center.to_string())
            .await?
            .unwrap();
        actor_ref
            .tell(&CenterMessage::Register {
                server_role_id: self.server_role_id(),
                peer_id: actor_swarm.local_peer_id().clone(),
            })
            .await?;
        Ok(actor_ref)
    }
}
