use common::config::{GameServerConfig, GlobalConfig, ServerRoleId};
use kameo::{Actor, RemoteActor};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub mod node;

#[derive(RemoteActor)]
pub struct CenterActor {
    global_config: Arc<GlobalConfig>,
}

impl CenterActor {
    fn new(global_config: Arc<GlobalConfig>) -> Self {
        Self { global_config }
    }
}

impl Actor for CenterActor {
    type Error = CenterActorError;
}
#[derive(Debug, Clone)]
pub struct CenterActorError {
    error: String,
}
impl Display for CenterActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}
