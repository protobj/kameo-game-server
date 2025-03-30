use crate::DataError;
use crate::login::node::LoginActor;
use protocol::login_cmd::{LoginReq, LoginRsp, RegisterReq, RegisterRsp};

pub async fn login_req(actor: &mut LoginActor, msg: LoginReq) -> Result<LoginRsp, DataError> {
    let account = msg.account;
    let server_id = msg.server_id;
    let mut redis_conn = &mut actor.redis_conn;
    Ok(LoginRsp {})
}
pub async fn register_req(
    actor: &mut LoginActor,
    msg: RegisterReq,
) -> Result<RegisterRsp, DataError> {
    Ok(RegisterRsp {})
}
