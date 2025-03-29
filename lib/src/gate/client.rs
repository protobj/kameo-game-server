use crate::game::GameActor;
use crate::gate::packet::{Encoder, Packet, Type};
use crate::gate::server::NetServerSignal;
use crate::login::LoginActor;
use crate::world::WorldActor;
use crate::Gate2OtherReq;
use bytes::Bytes;
use kameo::actor::{ActorRef, RemoteActorRef};
use kameo::message::{Context, Message};
use kameo::Actor;
use message_io::network::{Endpoint, SendStatus};
use message_io::node::NodeHandler;
use protocol::base_cmd::ErrorRsp;
use protocol::cmd::Cmd;
use std::time::Duration;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

pub struct ClientActor {
    last_heartbeat: u128,
    endpoint: Endpoint,
    handler: NodeHandler<NetServerSignal>,
}

impl ClientActor {
    pub fn new(endpoint: Endpoint, handler: NodeHandler<NetServerSignal>) -> Self {
        Self {
            endpoint,
            handler,
            last_heartbeat: 0,
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
    /// request|notify
    pub(crate) async fn handle_data(
        &mut self,
        actor_ref: ActorRef<ClientActor>,
        packet: Packet,
        request: bool,
    ) {
        let result = self.inner_handle_data(actor_ref, packet, request).await;
        match result {
            Ok(p) => match p {
                None => {}
                Some(p) => {
                    self.send(p);
                }
            },
            Err(e) => self.response(
                Cmd::CmdErrorRsp as i32,
                protocol::helper::encode(e).expect(
                    format!("endpoint:{} Failed to encode response: ", self.endpoint).as_str(),
                ),
            ),
        }
    }

    async fn inner_handle_data(
        &mut self,
        actor_ref: ActorRef<ClientActor>,
        packet: Packet,
        request: bool,
    ) -> Result<Option<Packet>, ErrorRsp> {
        let mut bytes = packet.data.unwrap();
        let cmd = packet.cmd;
        let cmd = match Cmd::try_from(cmd) {
            Ok(x) => x,
            Err(e) => {
                //发送了错误的命令,不管请求还是推送都返回错误
                return Err(ErrorRsp {
                    cmd,
                    code: protocol::error::Error::UnknownCommandError as i32,
                    message: e.to_string(),
                }
                .into());
            }
        };

        let rsp = match cmd {
            c if forward_to_login(c) => {
                let login_actor_ref = RemoteActorRef::<LoginActor>::lookup("login-1")
                    .await
                    .unwrap()
                    .unwrap();
                login_actor_ref.ask(&Gate2OtherReq { cmd, bytes }).await
            }
            c if forward_to_world(c) => {
                let world_actor_ref = RemoteActorRef::<WorldActor>::lookup("world-1")
                    .await
                    .unwrap()
                    .unwrap();
                world_actor_ref.ask(&Gate2OtherReq { cmd, bytes }).await
            }
            c if forward_to_game(c) => {
                let game_actor_ref = RemoteActorRef::<GameActor>::lookup("game-1")
                    .await
                    .unwrap()
                    .unwrap();
                game_actor_ref.ask(&Gate2OtherReq { cmd, bytes }).await
            }
            _ => {
                //剩下的都发到game服的PlayerActor
                let game_actor_ref = RemoteActorRef::<GameActor>::lookup("player-1")
                    .await
                    .unwrap()
                    .unwrap();
                game_actor_ref.ask(&Gate2OtherReq { cmd, bytes }).await
            }
        };
        match rsp {
            Ok(x) => Ok(Some(Packet::new_data(
                Type::Response,
                x.cmd as i32,
                x.bytes,
            ))),
            Err(e) => match e {
                _ => Err(ErrorRsp {
                    cmd: cmd as i32,
                    code: protocol::error::Error::ServerInternalError as i32,
                    message: e.to_string(),
                }),
            },
        }
    }
}
fn forward_to_world(cmd: Cmd) -> bool {
    return true;
}
fn forward_to_game(cmd: Cmd) -> bool {
    return true;
}

fn forward_to_login(cmd: Cmd) -> bool {
    return true;
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
                    self.handle_data(ctx.actor_ref(), packet, true).await;
                }
                Type::Notify => {
                    self.handle_data(ctx.actor_ref(), packet, false).await;
                }
                _ => {}
            },
            ClientMessage::SendPacket(packet) => self.send(packet),
        }
    }
}
