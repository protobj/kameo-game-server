use common::config::{GlobalConfig, ServerRoleId};
use common::service::{Node, Signal};
use std::sync::Arc;
use tokio::sync::watch::Receiver;

pub struct WorldNode;
#[async_trait::async_trait]
impl Node for WorldNode {
    async fn start(&self, global_config: Arc<GlobalConfig>, server_role_id: ServerRoleId) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
