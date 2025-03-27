use common::config::ServerRoleId;
use kameo::remote::ActorSwarm;
use kameo::remote::dial_opts::DialOpts;
use std::ops::Deref;
use tokio::sync::watch::Receiver;

mod game;
mod gate;
mod login;
mod world;

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
                    tracing::error!("signal recv err :{}", self.server_role_id());
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
            "ActorSwarm[{}] listening addr:{} id:{}",
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
}
pub mod prelude {
    pub use crate::game::GameNode;
    pub use crate::gate::GateNode;
    pub use crate::login::LoginNode;
    pub use crate::world::WorldNode;
}
