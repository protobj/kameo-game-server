use crate::gate::GateActor;
use kameo::message::{Context, Message};
use kameo::remote_message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GateMessage {
    Hello,
}
#[remote_message("gate-message")]
impl Message<GateMessage> for GateActor {
    type Reply = String;

    fn handle(
        &mut self,
        msg: GateMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> impl Future<Output = Self::Reply> + Send {
        async { String::from("hahahaah") }
    }
}
