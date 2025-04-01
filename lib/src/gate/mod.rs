use crate::gate::net_server::{NetServer, NetServerSignal};
use common::config::{GateServerConfig, GlobalConfig, ServerRoleId};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::{Actor, RemoteActor};
use message_io::node::{NodeHandler, NodeTask};
use std::fmt::Display;
use std::sync::Arc;

pub mod client;
pub mod net_server;
pub mod node;
pub mod packet;
#[derive(RemoteActor)]
pub struct GateActor {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    gate_config: GateServerConfig,
    node_task: Option<(NodeTask, NodeHandler<NetServerSignal>)>,
}

impl GateActor {
    pub fn new(
        global_config: Arc<GlobalConfig>,
        role_id: ServerRoleId,
        gate_config: GateServerConfig,
    ) -> Self {
        Self {
            role_id,
            global_config,
            gate_config,
            node_task: None,
        }
    }
}
impl Actor for GateActor {
    type Error = GateActorError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .register(self.role_id.to_string().as_str())
            .await
            .map_err(|e| {
                tracing::error!("GateActor register remote fail:{}", e);
                GateActorError::RegisterRemoteFail(e.to_string())
            })?;
        //启动监听
        let net_server = NetServer::new(
            self.gate_config.out_tcp_port,
            self.gate_config.out_ws_port,
            self.gate_config.out_udp_port,
        )
        .map_err(|e| {
            tracing::error!("GateActor ListenNetFail fail:{}", e);
            GateActorError::ListenNetFail(e.to_string())
        })?;

        let node_task = net_server.run();
        self.node_task = Some(node_task);

        Ok(())
    }
    fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async {
            if let Some(task) = self.node_task.take() {
                task.1.stop();
                drop(task.0);
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum GateActorError {
    ConnectFail(String),
    RegisterRemoteFail(String),
    ListenNetFail(String),
}

impl Display for GateActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateActorError::ConnectFail(s) => {
                f.write_str(format!("ConnectFail reason:{}", s).as_str())
            }
            GateActorError::RegisterRemoteFail(s) => {
                f.write_str(format!("RegisterRemoteFail reason:{}", s).as_str())
            }
            GateActorError::ListenNetFail(s) => {
                f.write_str(format!("ListenNetFail reason:{}", s).as_str())
            }
        }
    }
}
