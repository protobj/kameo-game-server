use crate::gate::GateActor;
use crate::node::Node;
use common::config::{GlobalConfig, ServerRoleId};
use kameo::actor::ActorRef;
use std::sync::Arc;

pub struct GateNode {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    gate_ref: Option<ActorRef<GateActor>>,
}
impl GateNode {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
            gate_ref: None,
        }
    }
}

#[async_trait::async_trait]
impl Node for GateNode {
    async fn start(&mut self) -> anyhow::Result<()> {
        let global_config = self.global_config.clone();
        let role_id = self.role_id.clone();
        let gate_config = global_config.find_gate_config(role_id.1);
        let gate_config = match gate_config {
            None => return Err(anyhow::anyhow!("Gate config not found:{}", role_id)),
            Some(x) => x,
        };

        self.start_actor_swarm(
            gate_config.in_address.clone(),
            vec![global_config.center_in_address().to_string()],
        )
        .await?;
        //集群启动好后,启动GateActor
        let gate_ref = kameo::spawn(GateActor::new(global_config, role_id, gate_config));
        let result = gate_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "GateActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.gate_ref = Some(gate_ref);
        tracing::info!("GateActor start success:{}", self.role_id);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        let actor_ref = self.gate_ref.take().unwrap();
        //停止actor
        actor_ref.kill();
        actor_ref.wait_for_stop().await;
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.role_id.clone()
    }
}
