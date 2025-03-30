use crate::DataError::{Other, RspError};
use bytes::{Bytes, BytesMut};
use kameo::Reply;
use prost::Message;
use protocol::base_cmd::BaseError::ErrorServerInternal;
use protocol::base_cmd::ErrorRsp;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use thiserror::Error;

mod discovery;
mod game;
mod gate;
mod login;
pub mod node;
mod registry;
mod world;
pub mod prelude {
    pub use crate::game::node::GameNode;
    pub use crate::gate::node::GateNode;
    pub use crate::login::node::LoginNode;
    pub use crate::world::node::WorldNode;
}

#[derive(Reply, serde::Serialize, serde::Deserialize)]
pub struct ServerMessage {
    pub(crate) cmd: i32,
    pub(crate) data: Bytes,
}

#[derive(Error, Debug, Deserialize, Serialize)]
pub enum DataError {
    //请求错误
    #[error("data error:{0:?} msg:{1}")]
    RspError(i32, String),
    #[error("other error:{0}")]
    Other(String),
}

impl From<DataError> for ErrorRsp {
    fn from(value: DataError) -> Self {
        match value {
            RspError(code, msg) => ErrorRsp {
                cmd: 0,
                code,
                message: msg,
            },
            Other(msg) => ErrorRsp {
                cmd: 0,
                code: ErrorServerInternal as i32,
                message: msg,
            },
        }
    }
}

pub type ReqResult = Result<ServerMessage, DataError>;
pub type NtfResult = Result<(), DataError>;

pub struct Req<T: Message>(pub T);

pub fn encode<T: Message>(message: T) -> Result<Bytes, DataError> {
    let mut bytes_mut = BytesMut::new();
    message
        .encode(&mut bytes_mut)
        .map_err(|x| DataError::Other(x.to_string()))?;
    Ok(bytes_mut.freeze())
}

pub fn decode<T: Message + Default>(bytes: Bytes) -> Result<T, DataError> {
    T::decode(bytes.as_ref()).map_err(|x| DataError::Other(x.to_string()))
}
#[macro_export]
macro_rules! proc_req {
    ($msg:ident,$self:ident,$handler:ident, $req_type:ty) => {{
        let rsp = $handler($self, crate::decode::<$req_type>($msg.data)?).await?;
        Ok(ServerMessage {
            cmd: rsp.cmd(),
            data: crate::encode(rsp)?,
        })
    }};
}
#[macro_export]
macro_rules! proc_ntf {
    ($msg:ident,$self:ident,$handler:ident, $req_type:ty) => {{
        $handler($self, crate::decode::<$req_type>($msg.data)?).await?;
    }};
}
