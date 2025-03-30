use crate::game::GameActor;
use crate::{DataError, ServerMessage};
use kameo::message::{Context, Message};
use kameo::{remote_message, Actor, RemoteActor};
use protocol::base_cmd::BaseError::ErrorUnknownCommand;

#[derive(RemoteActor)]
pub struct PlayerActor;

impl Actor for PlayerActor {
    type Error = ();
}

#[remote_message("Gate2OtherReq")]
impl Message<ServerMessage> for PlayerActor {
    type Reply = Result<ServerMessage, DataError>;
    async fn handle(
        &mut self,
        msg: ServerMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg.cmd {
            _ => Err(DataError::RspError(
                 ErrorUnknownCommand as i32,
                format!("UnknownCommandError:{}", msg.cmd),
            )),
        }
    }
}
