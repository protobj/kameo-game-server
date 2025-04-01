use crate::center::CenterActor;
use crate::node::Node;
use common::config::{GlobalConfig, ServerRole, ServerRoleId};
use kameo::actor::ActorRef;
use std::sync::Arc;

pub struct CenterNode {
    global_config: Arc<GlobalConfig>,
    center_ref: Option<ActorRef<CenterActor>>,
}
impl CenterNode {
    pub fn new(global_config: Arc<GlobalConfig>) -> Self {
        Self {
            global_config,
            center_ref: None,
        }
    }
}

#[async_trait::async_trait]
impl Node for CenterNode {
    async fn start(&mut self) -> anyhow::Result<()> {
        self.start_actor_swarm(self.global_config.center_in_address().to_string(), vec![])
            .await?;
        //集群启动好后,启动CenterActor
        let center_ref = kameo::spawn(CenterActor::new(self.global_config.clone()));
        let result = center_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "CenterActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.center_ref = Some(center_ref);

        tracing::info!("CenterActor start success");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        let actor_ref = self.center_ref.take().unwrap();
        //停止actor
        actor_ref.kill();
        actor_ref.wait_for_stop().await;
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        ServerRoleId(ServerRole::Center, 0)
    }
}
