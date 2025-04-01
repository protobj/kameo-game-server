use crate::game::GameActor;
use crate::gate::net_server::NetServerSignal;
use crate::gate::packet::{Encoder, Packet, Type};
use crate::gate::GateActor;
use crate::login::node::LoginActor;
use crate::world::WorldActor;
use crate::{DataError, ServerMessage};
use bytes::Bytes;
use common::config::ServerRole;
use kameo::actor::{ActorRef, RemoteActorRef};
use kameo::message::{Context, Message};
use kameo::Actor;
use message_io::network::{Endpoint, SendStatus};
use message_io::node::NodeHandler;
use protocol::base_cmd::BaseCmd::CmdErrorRsp;
use protocol::base_cmd::BaseError::{ErrorFunctionNotImpliment, ErrorServerInternal};
use protocol::base_cmd::ErrorRsp;
use protocol::login_cmd::{LoginReq, RegisterReq};
use std::ops::Deref;
use std::time::Duration;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub struct ClientActor {
    last_heartbeat: u128,
    endpoint: Endpoint,
    handler: NodeHandler<NetServerSignal>,
    account: Option<String>,
}

impl ClientActor {
    pub fn new(
        endpoint: Endpoint,
        handler: NodeHandler<NetServerSignal>,
    ) -> Self {
        Self {
            endpoint,
            handler,
            last_heartbeat: 0,
            account: None,
        }
    }

    pub fn push(&mut self, cmd: i32, bytes: Bytes) {
        self.send(Packet::new_data(Type::Push, cmd, bytes));
    }

    pub fn response(&mut self, cmd: i32, bytes: Bytes) {
        self.send(Packet::new_data(Type::Response, cmd, bytes));
    }

    fn send(&mut self, packet: Packet) {
        let typ = packet.r#type.to_string();
        let bytes = Encoder::encode(packet);
        let status = self.handler.network().send(self.endpoint, bytes.as_ref());
        if status != SendStatus::Sent {
            tracing::error!(
                "endpoint:{:?} Failed to send packet: {:?}",
                self.endpoint,
                typ
            );
        }
    }
    pub(crate) async fn handle_req(&mut self, packet: Packet) {
        let cmd = packet.cmd;
        let result = self.inner_handle_req(packet).await;
        match result {
            Ok(p) => {
                self.send(p);
            }
            Err(e) => {
                let mut rsp: ErrorRsp = e.into();
                rsp.cmd = cmd;
                self.response(
                    ErrorRsp::CMD,
                    crate::encode(rsp).expect(
                        format!("endpoint:{} Failed to encode response: ", self.endpoint).as_str(),
                    ),
                );
            }
        }
    }
    pub(crate) async fn handle_ntf(&mut self, packet: Packet) {
        let bytes = packet.data;
        let cmd = packet.cmd;

        let result = match cmd {
            LoginReq::CMD | RegisterReq::CMD => self.ntf(ServerRole::Login, cmd, bytes).await,
            c if forward_to_world(c) => self.ntf(ServerRole::World, cmd, bytes).await,
            c if forward_to_game(c) => self.ntf(ServerRole::Game, cmd, bytes).await,
            _ => self.ntf(ServerRole::Game, cmd, bytes).await,
        };
        if let Err(e) = result {
            tracing::error!("cmd:{} internal error: {:?}", cmd, e);
        }
    }

    async fn inner_handle_req(&mut self, packet: Packet) -> Result<Packet, DataError> {
        let bytes = packet.data;
        let cmd = packet.cmd;

        match cmd {
            LoginReq::CMD | RegisterReq::CMD => self.ask(ServerRole::Login, cmd, bytes).await,
            c if forward_to_world(c) => self.ask(ServerRole::World, cmd, bytes).await,
            c if forward_to_game(c) => self.ask(ServerRole::Game, cmd, bytes).await,
            _ => self.ask(ServerRole::Game, cmd, bytes).await,
        }
        .map(|x| Packet::new_data(Type::Response, x.cmd, x.data))
    }

    async fn ask(
        &self,
        role: ServerRole,
        cmd: i32,
        data: Bytes,
    ) -> Result<ServerMessage, DataError> {
        Err(DataError::RspError(
            ErrorFunctionNotImpliment as i32,
            "".to_string(),
        ))
    }
    async fn ntf(&self, role: ServerRole, cmd: i32, data: Bytes) -> Result<(), DataError> {
        Err(DataError::RspError(
            ErrorFunctionNotImpliment as i32,
            "".to_string(),
        ))
    }
}

fn forward_to_world(cmd: i32) -> bool {
    false
}
fn forward_to_game(cmd: i32) -> bool {
    false
}

fn forward_to_login(cmd: i32) -> bool {
    match cmd {
        LoginReq::CMD | RegisterReq::CMD => true,
        _ => false,
    }
}

impl Actor for ClientActor {
    type Error = ();
}

pub enum ClientMessage {
    ReceivePacket(Packet),
    SendPacket(Packet),
}
impl Message<ClientMessage> for ClientActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClientMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            ClientMessage::ReceivePacket(packet) => match packet.r#type {
                Type::Handshake => self.send(Packet::new_control(Type::HandshakeAck)),
                Type::Heartbeat => {
                    self.last_heartbeat = common::time::now_ms();
                    self.handler.signals().send_with_timer(
                        NetServerSignal::SendPacket(
                            self.endpoint,
                            Packet::new_control(Type::Heartbeat),
                        ),
                        HEARTBEAT_INTERVAL,
                    );
                }
                Type::Request => {
                    self.handle_req(packet).await;
                }
                Type::Notify => {
                    self.handle_ntf(packet).await;
                }
                _ => {}
            },
            ClientMessage::SendPacket(packet) => self.send(packet),
        }
    }
}