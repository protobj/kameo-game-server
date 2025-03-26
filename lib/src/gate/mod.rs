use common::config::{GlobalConfig, ServerRoleId};
use common::service::{Node, Signal};
use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::remote::ActorSwarm;
use std::sync::Arc;
use tokio::sync::watch::Receiver;

pub struct GateNode;
#[async_trait::async_trait]
impl Node for GateNode {
    async fn start(&self, global_config: Arc<GlobalConfig>, server_role_id: ServerRoleId) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct GateActor {
    role_id: ServerRoleId,
    global_config: Arc<GlobalConfig>,
}

impl Actor for GateActor {
    type Mailbox = UnboundedMailbox<Self>;
    type Error = anyhow::Error;

    fn on_start(
        &mut self,
        actor_ref: ActorRef<Self>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async {
            let global_config = &self.global_config;
            let role_id = &self.role_id;
            let gate_config = global_config.find_gate_config(role_id.1);
            let gate_config = match gate_config {
                None => return Err(anyhow::anyhow!("Gate config not found")),
                Some(x) => x,
            };
            ActorSwarm::bootstrap()
                .unwrap()
                .listen_on((gate_config.in_address).parse()?)
                .await?;

            Ok(())
        }
    }
}
