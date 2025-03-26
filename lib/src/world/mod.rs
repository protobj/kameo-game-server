use common::config::{GlobalConfig, ServerRoleId};
use std::sync::Arc;
use kameo::Actor;
use kameo::mailbox::unbounded::UnboundedMailbox;
use tokio::sync::watch::Receiver;
use crate::Node;

pub struct WorldActor {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
}

impl WorldActor {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
        }
    }
}


#[async_trait::async_trait]
impl Node for WorldActor {
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

impl Actor for WorldActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}