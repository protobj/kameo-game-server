use crate::Node;
use crate::gate::server::{NetServer, NetServerSignal};
use common::config::{GateServerConfig, GlobalConfig, ServerRoleId};
use kameo::actor::{ActorRef, PreparedActor, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::message::{Context, Message};
use kameo::{Actor, RemoteActor, remote_message};
use message_io::node::{NodeHandler, NodeTask};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

pub mod server;
pub mod packet;
pub mod client;
pub mod server_message;
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
            global_config.find_all_in_address(),
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
