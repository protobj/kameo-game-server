use crate::Node;
use common::config::{GlobalConfig, ServerRoleId};
use kameo::Actor;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::remote::ActorSwarm;
use std::sync::Arc;
use tokio::sync::watch::Receiver;

pub struct LoginActor {
    config: Arc<GlobalConfig>,
    server_role_id: ServerRoleId,
}

impl LoginActor {
    pub fn new(config: Arc<GlobalConfig>, server_role_id: ServerRoleId) -> Self {
        Self {
            config,
            server_role_id,
        }
    }
}

#[async_trait::async_trait]
impl Node for LoginActor {
    async fn start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.server_role_id.clone()
    }
}
impl Actor for LoginActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;
}
