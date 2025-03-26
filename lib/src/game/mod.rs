use common::config::{GlobalConfig, ServerRoleId};
use common::service::Node;
use std::sync::Arc;

pub struct GameNode;
#[async_trait::async_trait]
impl Node for GameNode {
    async fn start(
        &self,
        global_config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
