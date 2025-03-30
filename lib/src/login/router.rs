use crate::login::login_handler::{login_req, register_req};
use crate::login::node::LoginActor;
use crate::{proc_req, DataError, ServerMessage};
use kameo::message::{Context, Message};
use kameo::remote_message;
use protocol::login_cmd::{LoginReq, RegisterReq};
#[remote_message("Gate2LoginReq")]
impl Message<ServerMessage> for LoginActor {
    type Reply = Result<ServerMessage, DataError>;

    async fn handle(
        &mut self,
        msg: ServerMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg.cmd {
            LoginReq::CMD => proc_req!(msg, self, login_req, LoginReq),
            RegisterReq::CMD => proc_req!(msg, self, register_req, RegisterReq),
            _ => Err(DataError::Other("not found handler".to_string())),
        }
    }
}
