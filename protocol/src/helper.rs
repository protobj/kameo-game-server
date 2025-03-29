use crate::base_cmd::ErrorRsp;
use prost::bytes::{Bytes, BytesMut};
use prost::{DecodeError, EncodeError, Message};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub fn encode<T: Message>(message: T) -> Result<Bytes, EncodeError> {
    let mut bytes_mut = BytesMut::new();
    message.encode(&mut bytes_mut)?;
    Ok(bytes_mut.freeze())
}

pub fn decode<T: Message + Default>(bytes: Bytes) -> Result<T, DecodeError> {
    T::decode(bytes.as_ref())
}

pub fn build_error(cmd: i32, err: crate::error::Error, msg: String) -> Result<Bytes, EncodeError> {
    encode(ErrorRsp {
        cmd,
        code: err as i32,
        message: msg,
    })
}
impl Display for ErrorRsp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "request:{} error code:{}  message:{}",
            self.cmd, self.code, self.message
        )
    }
}
impl std::error::Error for ErrorRsp {}

#[derive(Serialize, Deserialize)]
pub struct Empty;
