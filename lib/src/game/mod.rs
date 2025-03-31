use crate::gate::{GateActor, GateActorError};
use crate::node::Node;
use crate::{DataError, ServerMessage};
use common::config::{GameServerConfig, GlobalConfig, ServerRoleId};
use kameo::actor::{ActorRef, RemoteActorRef};
use kameo::message::{Context, Message};
use kameo::{Actor, RemoteActor, remote_message};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
pub mod node;
pub mod player;
pub mod router;
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

impl Actor for GameActor {
    type Error = GameActorError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .register(self.role_id.to_string().as_str())
            .await
            .map_err(|e| {
                tracing::error!("GameActor register remote fail:{}", e);
                GameActorError::RegisterRemoteFail(e.to_string())
            })?;
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
