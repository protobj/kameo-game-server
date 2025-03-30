use backon::ExponentialBuilder;
use crate::game::GameActor;
use crate::gate::GateActor;
use crate::login::node::LoginActor;
use crate::world::WorldActor;
use crate::{DataError, ServerMessage};
use backon::Retryable;
use bytes::Bytes;
use common::config::{ServerRole, ServerRoleId};
use dashmap::DashMap;
use kameo::actor::RemoteActorRef;
use kameo::error::RemoteSendError;
use std::time::Duration;
use crossbeam::queue::SegQueue;
use kameo::Actor;
use lazy_static::lazy_static;
use scc::Bag;

pub struct NodeWrapper<T: Actor> {
    pub role_id: ServerRoleId,
    pub access: i32,
    pub actor_ref: RemoteActorRef<T>,
}

pub enum NodeGroup<T: Actor> {
    Random(Bag<NodeWrapper<T>>),
    Direct(DashMap<i32, NodeWrapper<T>>),
    Lease(SegQueue<NodeWrapper<T>>),
}

lazy_static! {
    static ref LOGIN_NODES: NodeGroup<LoginActor> = NodeGroup::Lease(SegQueue::new());
    static ref WORLD_NODES: NodeGroup<WorldActor> = NodeGroup::Direct(DashMap::new());
    static ref GAME_NODES: NodeGroup<GameActor> = NodeGroup::Direct(DashMap::new());
}
pub struct NodeManager {
    login_nodes: DashMap<ServerRoleId, RemoteActorRef<LoginActor>>,
    world_nodes: DashMap<ServerRoleId, RemoteActorRef<WorldActor>>,
    game_nodes: DashMap<ServerRoleId, RemoteActorRef<GameActor>>,
    gate_nodes: DashMap<ServerRoleId, RemoteActorRef<GateActor>>,
}
macro_rules! find_node {
    ($name:ident,$ActorType:ty) => {
        impl NodeManager {
            pub async fn $name(node_name: &str) -> anyhow::Result<RemoteActorRef<$ActorType>> {
                Ok(RemoteActorRef::<$ActorType>::lookup(&node_name)
                    .await?
                    .unwrap())
            }
        }
    };
}
find_node!(find_login_node, LoginActor);
find_node!(find_world_node, WorldActor);
find_node!(find_game_node, GameActor);
find_node!(find_gate_node, GateActor);

macro_rules! get_with_retry {
    ($name:ident,$role_id:ident) => {
        (|| async { NodeManager::$name(&$role_id).await })
            .retry(ExponentialBuilder::new().with_max_delay(Duration::from_millis(500)))
            .when(|e| e.to_string() == "retryable")
            .await
            .map_err(|e| DataError::Other(e.to_string()))?
    };
}

impl NodeManager {
    pub fn new() -> Self {
        Self {
            login_nodes: DashMap::new(),
            world_nodes: DashMap::new(),
            game_nodes: DashMap::new(),
            gate_nodes: DashMap::new(),
        }
    }
    pub async fn ask(
        &mut self,
        server_role: ServerRoleId,
        cmd: i32,
        data: Bytes,
    ) -> Result<ServerMessage, DataError> {
        let role_id = server_role.to_string();
        let result = match server_role.0 {
            ServerRole::Login => {
                let actor_ref = get_with_retry!(find_login_node, role_id);
                actor_ref.ask(&ServerMessage { cmd, data }).await
            }
            ServerRole::Game => {
                let actor_ref = get_with_retry!(find_game_node, role_id);
                actor_ref.ask(&ServerMessage { cmd, data }).await
            }
            ServerRole::World => {
                let actor_ref = get_with_retry!(find_world_node, role_id);
                actor_ref.ask(&ServerMessage { cmd, data }).await
            }
            _ => {
                panic!("Unsupported server role");
            }
        };

        result.map_err(|e| {
            return match e {
                RemoteSendError::HandlerError(e) => e,
                e => DataError::Other(e.to_string()),
            };
        })
    }
}