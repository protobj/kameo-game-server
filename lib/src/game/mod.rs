use crate::Node;
use common::config::{GlobalConfig, ServerRoleId};
use kameo::Actor;
use kameo::mailbox::unbounded::UnboundedMailbox;
use std::sync::Arc;

pub struct GameActor {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
}
impl GameActor {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
        }
    }
}
#[async_trait::async_trait]
impl Node for GameActor {
    async fn start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.role_id.clone()
    }
}
impl Actor for GameActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}
