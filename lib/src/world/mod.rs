use crate::node::Node;
use crate::{DataError, ServerMessage};
use common::config::{GlobalConfig, ServerRoleId, WorldServerConfig};
use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use kameo::{remote_message, Actor, RemoteActor};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
pub mod node;
pub mod router;
#[derive(RemoteActor)]
pub struct WorldActor {
    config: Arc<GlobalConfig>,
    server_role_id: ServerRoleId,
    world_config: WorldServerConfig,
}

impl WorldActor {
    pub fn new(
        config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
        world_config: WorldServerConfig,
    ) -> Self {
        Self {
            config,
            server_role_id,
            world_config,
        }
    }
}

impl Actor for WorldActor {
    type Error = WorldActorError;
}
#[derive(Debug, Clone)]
pub enum WorldActorError {}
impl Display for WorldActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorldActorError")
    }
}

