use crate::game::GameActor;
use crate::node::Node;
use common::config::{GlobalConfig, ServerRoleId};
use kameo::actor::ActorRef;
use std::sync::Arc;

pub struct GameNode {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    game_ref: Option<ActorRef<GameActor>>,
}

impl GameNode {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
            game_ref: None,
        }
    }
}

#[async_trait::async_trait]
impl Node for GameNode {
    async fn start(&mut self) -> anyhow::Result<()> {
        let global_config = self.global_config.clone();
        let role_id = self.role_id.clone();
        let game_config = global_config.find_game_config(role_id.1);
        let game_config = match game_config {
            None => return Err(anyhow::anyhow!("Game config not found:{}", role_id)),
            Some(x) => x,
        };

        self.start_actor_swarm(
            game_config.in_address.clone(),
            vec![global_config.center_in_address().to_string()],
        )
        .await?;
        //集群启动好后,启动GameActor
        let game_ref = kameo::spawn(GameActor::new(global_config, role_id, game_config));
        let result = game_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "GameActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.game_ref = Some(game_ref);
        self.connect_center().await?;
        tracing::info!("GameActor start success:{}", self.role_id);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        let actor_ref = self.game_ref.take().unwrap();
        //停止actor
        actor_ref.kill();
        actor_ref.wait_for_stop().await;
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.role_id.clone()
    }
}
