use crate::gate::session::ClientSession;
use crate::Node;
use common::config::{GateServerConfig, GlobalConfig, ServerRoleId};
use kameo::actor::{ActorRef, PreparedActor, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::mailbox::unbounded::UnboundedMailbox;
use kameo::message::{Context, Message};
use kameo::{remote_message, Actor, RemoteActor};
use network::tcp::listener::Listener;
use network::tcp::session::TcpSession;
use network::websocket::session::WsSession;
use network::{LogicMessage, MessageHandler};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

pub mod message;
pub mod session;

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
    tcp_ref: Option<ActorRef<Listener<TcpSession<TcpMessageHandler>>>>,
    ws_ref: Option<ActorRef<Listener<WsSession<WebSocketMessageHandler>>>>,
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
            tcp_ref: None,
            ws_ref: None,
        }
    }

    async fn listen_tcp(&mut self) -> Option<ActorRef<Listener<TcpSession<TcpMessageHandler>>>> {
        let port = self.gate_config.out_tcp_port;
        if port == 0 {
            return None;
        }

        let tcp_ref = kameo::spawn(Listener::new(port, move |info, stream| async move {
            //先初始化actor, 再spawn
            let prepared_actor = PreparedActor::<TcpSession<TcpMessageHandler>>::new();
            let actor_ref = prepared_actor.actor_ref().clone();
            prepared_actor.spawn(TcpSession::new(
                info,
                stream,
                TcpMessageHandler {
                    gate_session: ClientSession::new_tcp(Some(actor_ref.clone())),
                },
            ));
            Ok(actor_ref)
        }));
        tcp_ref.wait_startup().await;
        Some(tcp_ref)
    }

    async fn listen_websocket(
        &mut self,
    ) -> Option<ActorRef<Listener<WsSession<WebSocketMessageHandler>>>> {
        let port = self.gate_config.out_ws_port;
        if port == 0 {
            return None;
        }

        let ws_ref = kameo::spawn(Listener::new(port, move |info, stream| async move {
            //先初始化actor, 再spawn
            let prepared_actor = PreparedActor::<WsSession<WebSocketMessageHandler>>::new();
            let actor_ref = prepared_actor.actor_ref().clone();
            WsSession::new(
                info,
                stream,
                WebSocketMessageHandler {
                    gate_session: ClientSession::new_ws(Some(actor_ref.clone())),
                },
            )
            .await
            .map(move |ws_session| {
                prepared_actor.spawn(ws_session);
                actor_ref
            })
        }));
        ws_ref.wait_startup().await;
        Some(ws_ref)
    }
}
impl Actor for GateActor {
    type Mailbox = UnboundedMailbox<Self>;
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
        //tcp
        self.tcp_ref = self.listen_tcp().await;
        //websocket
        self.ws_ref = self.listen_websocket().await;
        Ok(())
    }
    fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async {
            if let Some(tcp_ref) = &mut self.tcp_ref {
                tcp_ref.kill();
                tcp_ref.wait_for_stop().await;
            }
            if let Some(ws_ref) = &mut self.ws_ref {
                ws_ref.kill();
                ws_ref.wait_for_stop().await;
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum GateActorError {
    ConnectFail(String),
    RegisterRemoteFail(String),
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
        }
    }
}

pub(crate) struct TcpMessageHandler {
    gate_session: ClientSession,
}
impl MessageHandler for TcpMessageHandler {
    type Actor = TcpSession<Self>;
    async fn message_read(
        &mut self,
        actor_ref: ActorRef<Self::Actor>,
        logic_message: LogicMessage,
    ) {
        let session = &mut self.gate_session;
        session.handle_message(logic_message).await;
    }
}

pub(crate) struct WebSocketMessageHandler {
    gate_session: ClientSession,
}
impl MessageHandler for WebSocketMessageHandler {
    type Actor = WsSession<Self>;
    async fn message_read(
        &mut self,
        actor_ref: ActorRef<Self::Actor>,
        logic_message: LogicMessage,
    ) {
        let session = &mut self.gate_session;
        session.handle_message(logic_message).await;
    }
}
