use crate::node::Node;
use crate::world::WorldActor;
use common::config::{GlobalConfig, ServerRoleId};
use kameo::actor::ActorRef;
use std::sync::Arc;

pub struct WorldNode {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    world_ref: Option<ActorRef<WorldActor>>,
}
impl WorldNode {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
            world_ref: None,
        }
    }
}
#[async_trait::async_trait]
impl Node for WorldNode {
    async fn start(&mut self) -> anyhow::Result<()> {
        let global_config = self.global_config.clone();
        let role_id = self.role_id.clone();
        let world_config = global_config.find_world_config(role_id.1);
        let world_config = match world_config {
            None => return Err(anyhow::anyhow!("World config not found:{}", role_id)),
            Some(x) => x,
        };

        self.start_actor_swarm(
            world_config.in_address.clone(),
            vec![global_config.center_in_address().to_string()],
        )
        .await?;
        //集群启动好后,启动WorldActor
        let login_ref = kameo::spawn(WorldActor::new(global_config, role_id, world_config));
        let result = login_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "WorldActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.world_ref = Some(login_ref);
        tracing::info!("WorldActor start success:{}", self.role_id);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        let actor_ref = self.world_ref.take().unwrap();
        //停止actor
        actor_ref.kill();
        actor_ref.wait_for_stop().await;
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.role_id.clone()
    }
}
