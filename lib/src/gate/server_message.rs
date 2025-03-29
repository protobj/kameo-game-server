use crate::gate::GateActor;
use common::config::ServerRoleId;
use kameo::message::{Context, Message};
use kameo::remote_message;
use serde::{Deserialize, Serialize};
use crate::Gate2OtherRes;

#[derive(Serialize, Deserialize)]
pub struct RegisterGateMessageReq {
    up: bool,
    role: ServerRoleId,
}
#[derive(Serialize, Deserialize)]
pub struct RegisterGateMessageRsp {
    ok: bool,
}
#[remote_message("RegisterGateMessage")]
impl Message<RegisterGateMessageReq> for GateActor {
    type Reply = String;

    async fn handle(
        &mut self,
        msg: RegisterGateMessageReq,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
       ".".to_string()
    }
}
