use crate::game::{GameActor, GameActorError};
use crate::gate::GateActor;
use crate::login::node::LoginActor;
use crate::world::WorldActor;
use common::config::{GameServerConfig, GlobalConfig, ServerRole, ServerRoleId};
use kameo::actor::{ActorID, ActorRef, RemoteActorRef, WeakActorRef};
use kameo::error::{ActorStopReason, RegistryError};
use kameo::message::{Context, Message};
use kameo::{Actor, RemoteActor, remote_message};
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::sync::Arc;

pub mod node;
#[derive(Default)]
struct NodeContainer {
    peer_map: HashMap<PeerId, HashMap<String, Arc<Node>>>,
    role_map: HashMap<ServerRole, HashMap<String, Arc<Node>>>,
    game_map: HashMap<u32, NodeRef<GameActor>>,
    world_map: HashMap<u32, NodeRef<WorldActor>>,
    gate_map: HashMap<u32, NodeRef<GateActor>>,
    login_map: HashMap<u32, NodeRef<LoginActor>>,
}
unsafe impl Send for NodeContainer {}
unsafe impl Sync for NodeContainer {}
struct NodeRef<T: Actor> {
    node: Arc<Node>,
    actor_ref: RemoteActorRef<T>,
}
#[derive(Debug)]
struct Node {
    server_role_id: ServerRoleId,
    peer_id: PeerId,
}
#[derive(RemoteActor)]
pub struct CenterActor {
    _global_config: Arc<GlobalConfig>,
    node_container: NodeContainer,
}
macro_rules! handle_register {
    ($ActorType:ty,$map_name:ident,$peer_id:ident,$server_role:ident,$node:ident,$self:ident,$actor_ref:ident) => {
        let key = $node.server_role_id.to_string();
        let result = RemoteActorRef::<$ActorType>::lookup(&key).await;
        match result {
            Ok(r) => match r {
                None => {
                    tracing::error!("Actor:{:?} register error:None", $node);
                }
                Some(actor_ref) => {
                    $actor_ref
                        .link_remote(&actor_ref)
                        .await
                        .expect(format!("Actor:{:?} register error:None", $node).as_ref());
                    $self
                        .node_container
                        .role_map
                        .entry($server_role)
                        .or_insert_with(|| HashMap::new())
                        .insert(key.clone(), $node.clone());
                    $self
                        .node_container
                        .peer_map
                        .entry($peer_id)
                        .or_insert_with(|| HashMap::new())
                        .insert(key, $node.clone());

                    let id = $node.server_role_id.1;
                    let node_ref = NodeRef::<$ActorType> {
                        node: $node.clone(),
                        actor_ref,
                    };
                    $self.node_container.$map_name.insert(id, node_ref);
                    tracing::info!("Actor:{:?} register success", $node);
                }
            },
            Err(e) => {
                tracing::error!("Actor:{:?} register error:{}", $node, e);
            }
        }
    };
}
impl CenterActor {
    fn new(global_config: Arc<GlobalConfig>) -> Self {
        Self {
            _global_config: global_config,
            node_container: NodeContainer::default(),
        }
    }

    async fn register(
        &mut self,
        server_role_id: ServerRoleId,
        peer_id: PeerId,
        actor_ref: ActorRef<CenterActor>,
    ) {
        let server_role = server_role_id.0.clone();
        let node = Arc::new(Node {
            server_role_id,
            peer_id: peer_id.clone(),
        });

        match server_role {
            ServerRole::Game => {
                handle_register!(
                    GameActor,
                    game_map,
                    peer_id,
                    server_role,
                    node,
                    self,
                    actor_ref
                );
            }
            ServerRole::Gate => {
                handle_register!(
                    GateActor,
                    gate_map,
                    peer_id,
                    server_role,
                    node,
                    self,
                    actor_ref
                );
            }
            ServerRole::Login => {
                handle_register!(
                    LoginActor,
                    login_map,
                    peer_id,
                    server_role,
                    node,
                    self,
                    actor_ref
                );
            }
            ServerRole::World => {
                handle_register!(
                    WorldActor,
                    world_map,
                    peer_id,
                    server_role,
                    node,
                    self,
                    actor_ref
                );
            }
            _ => {}
        }
    }
    pub(crate) fn unregister(&mut self, server_role_id: ServerRoleId, peer_id: PeerId) {
        tracing::info!("Actor:{:?} peer_id:{} unregister", server_role_id, peer_id);
        self.node_container
            .peer_map
            .entry(peer_id)
            .or_insert_with(|| HashMap::new())
            .remove(&server_role_id.to_string());
        let role = server_role_id.0.clone();
        self.node_container
            .role_map
            .entry(role.clone())
            .or_insert_with(|| HashMap::new())
            .remove(&server_role_id.to_string());
        let id = server_role_id.1;
        match role {
            ServerRole::Login => {
                self.node_container.login_map.remove(&id);
            }
            ServerRole::Gate => {
                self.node_container.gate_map.remove(&id);
            }
            ServerRole::Game => {
                self.node_container.game_map.remove(&id);
            }
            ServerRole::World => {
                self.node_container.world_map.remove(&id);
            }
            _ => {}
        }
    }
}

impl Actor for CenterActor {
    type Error = CenterActorError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .register(&ServerRole::Center.to_string())
            .await
            .map_err(|e| {
                tracing::error!("GameActor register remote fail:{}", e);
                CenterActorError {
                    error: e.to_string(),
                }
            })?;
        Ok(())
    }
    async fn on_link_died(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        id: ActorID,
        reason: ActorStopReason,
    ) -> Result<ControlFlow<ActorStopReason>, Self::Error> {
        if let Some(peer_id) = id.peer_id() {
            tracing::error!("peer_id:{}  down", peer_id);
            //远程Actor断开,断开所有连接
            let option = self.node_container.peer_map.remove(&peer_id);
            if let Some(mut node_map) = option {
                for x in node_map.values_mut() {
                    self.unregister(x.server_role_id.clone(), peer_id.clone())
                }
            }
            //这里不需要通知其他Actor,由自身Actor建立Link
        }
        match &reason {
            ActorStopReason::Normal => Ok(ControlFlow::Continue(())),
            ActorStopReason::Killed
            | ActorStopReason::Panicked(_)
            | ActorStopReason::LinkDied { .. } => {
                Ok(ControlFlow::Break(ActorStopReason::LinkDied {
                    id,
                    reason: Box::new(reason),
                }))
            }
            ActorStopReason::PeerDisconnected => {
                Ok(ControlFlow::Break(ActorStopReason::PeerDisconnected))
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct CenterActorError {
    error: String,
}
impl Display for CenterActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}

#[derive(Deserialize, Serialize)]
pub enum CenterMessage {
    Register {
        server_role_id: ServerRoleId,
        peer_id: PeerId,
    },
    Unregister {
        server_role_id: ServerRoleId,
        peer_id: PeerId,
    },
}
#[remote_message("CenterMessage")]
impl Message<CenterMessage> for CenterActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: CenterMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            CenterMessage::Register {
                server_role_id,
                peer_id,
            } => {
                self.register(server_role_id, peer_id, ctx.actor_ref())
                    .await;
            }
            CenterMessage::Unregister {
                server_role_id,
                peer_id,
            } => {
                self.unregister(server_role_id, peer_id);
            }
        };
    }
}
#[derive(Deserialize, Serialize)]
pub enum SearchServerMessage {
    Ask { server_role: ServerRole },
    AskById { server_role_id: ServerRoleId },
}
#[remote_message("SearchServerMessage")]
impl Message<SearchServerMessage> for CenterActor {
    type Reply = String;

    async fn handle(
        &mut self,
        msg: SearchServerMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            SearchServerMessage::Ask { server_role, .. } => {
                let option = self.node_container.role_map.get(&server_role);
                if let Some(mut node_map) = option {
                    if node_map.is_empty() {
                        return "".to_string();
                    }
                    let server_role_id =
                        &node_map.values().into_iter().next().unwrap().server_role_id;
                    return server_role_id.to_string();
                }
                "".to_string()
            }
            SearchServerMessage::AskById { server_role_id, .. } => match server_role_id.0 {
                ServerRole::Login => self
                    .node_container
                    .login_map
                    .get(&server_role_id.1)
                    .map(|node| node.node.server_role_id.to_string())
                    .unwrap_or("".to_string()),
                ServerRole::Gate => self
                    .node_container
                    .gate_map
                    .get(&server_role_id.1)
                    .map(|node| node.node.server_role_id.to_string())
                    .unwrap_or("".to_string()),
                ServerRole::Game => self
                    .node_container
                    .game_map
                    .get(&server_role_id.1)
                    .map(|node| node.node.server_role_id.to_string())
                    .unwrap_or("".to_string()),
                ServerRole::World => self
                    .node_container
                    .world_map
                    .get(&server_role_id.1)
                    .map(|node| node.node.server_role_id.to_string())
                    .unwrap_or("".to_string()),
                _ => {
                    unreachable!("not support server");
                }
            },
        }
    }
}
