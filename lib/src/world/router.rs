use crate::world::WorldActor;
use crate::{DataError, ServerMessage};
use kameo::message::{Context, Message};
use kameo::remote_message;
use protocol::base_cmd::BaseError::ErrorFunctionNotImpliment;

#[remote_message("Gate2WorldReq")]
impl Message<ServerMessage> for WorldActor {
    type Reply = Result<ServerMessage, DataError>;
    async fn handle(
        &mut self,
        msg: ServerMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Err(DataError::RspError(
            ErrorFunctionNotImpliment as i32,
            "world node".to_string(),
        ))
    }
}
