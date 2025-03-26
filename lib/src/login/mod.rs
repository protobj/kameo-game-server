use common::config::{GlobalConfig, ServerRoleId};
use common::service::{Node, Signal};
use kameo::remote::ActorSwarm;
use std::sync::Arc;
use tokio::sync::watch::Receiver;

pub struct LoginNode;
#[async_trait::async_trait]
impl Node for LoginNode {

    async fn start(&self, global_config: Arc<GlobalConfig>, server_role_id: ServerRoleId) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
    Ok(())
    }
}
