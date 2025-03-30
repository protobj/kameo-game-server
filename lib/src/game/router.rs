use kameo::message::{Context, Message};
use kameo::remote_message;
use protocol::base_cmd::BaseError::ErrorFunctionNotImpliment;
use crate::game::GameActor;
use crate::{DataError, ServerMessage};

#[remote_message("Gate2GameReq")]
impl Message<ServerMessage> for GameActor {
    type Reply = Result<ServerMessage, DataError>;
    async fn handle(
        &mut self,
        msg: ServerMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Err(DataError::RspError(
            ErrorFunctionNotImpliment as i32,
            "game node".to_string(),
        ))
    }
}
