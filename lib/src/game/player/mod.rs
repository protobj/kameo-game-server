use crate::game::GameActor;
use crate::{Gate2OtherReq, Gate2OtherRes};
use kameo::message::{Context, Message};
use kameo::{Actor, RemoteActor, remote_message};

#[derive(RemoteActor)]
pub struct PlayerActor;

impl Actor for PlayerActor {
    type Error = ();
}

#[remote_message("Gate2OtherReq")]
impl Message<Gate2OtherReq> for PlayerActor {
    type Reply = Gate2OtherRes;
    async fn handle(
        &mut self,
        msg: Gate2OtherReq,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Gate2OtherRes {
            cmd: msg.cmd,
            bytes: msg.bytes,
        }
    }
}
