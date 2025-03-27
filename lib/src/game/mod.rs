use crate::gate::{message::GateMessage, GateActor, GateActorError};
use crate::Node;
use common::config::{GameServerConfig, GlobalConfig, ServerRoleId};
use kameo::actor::{ActorRef, RemoteActorRef};
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::{Actor, RemoteActor};
use std::fmt::{Display, Formatter};
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
#[derive(RemoteActor)]
pub struct GameActor {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    game_server_config: GameServerConfig,
}
impl GameActor {
    pub fn new(
        global_config: Arc<GlobalConfig>,
        role_id: ServerRoleId,
        game_server_config: GameServerConfig,
    ) -> Self {
        Self {
            global_config,
            role_id,
            game_server_config,
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
            global_config.find_all_in_address(),
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
impl Actor for GameActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = GameActorError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .register(self.role_id.to_string().as_str())
            .await
            .map_err(|e| {
                tracing::error!("GameActor register remote fail:{}", e);
                GameActorError::RegisterRemoteFail(e.to_string())
            })?;
        let gate_config_list = self.global_config.gate();
        for gate_config in gate_config_list {
            let name = &gate_config.unique_name();
            let remote_gate_ref = RemoteActorRef::<GateActor>::lookup(name)
                .await
                .map_err(|e| GameActorError::RegisterRemoteFail(e.to_string()))
                .unwrap()
                .unwrap();
            tracing::info!("connect gate :{:?}", remote_gate_ref);
            let result = remote_gate_ref.ask(&GateMessage::Hello).await;
            tracing::info!("gate:{} reply:{:?}", name, result);
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum GameActorError {
    RegisterRemoteFail(String),
}
impl Display for GameActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameActorError::RegisterRemoteFail(x) => {
                f.write_fmt(format_args!("Failed to register remote game config:{}", x))
            }
        }
    }
}
