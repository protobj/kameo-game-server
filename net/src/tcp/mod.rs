use bytes::{BufMut, Bytes, BytesMut};
use kameo::Actor;
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;

pub mod listener;
pub mod session;
pub mod client;
//自定义SessionActor
pub trait SessionActorTrait: Actor {
    fn new(stream: TcpStream) -> Self;
}

pub struct LogicMessage {
    pub ix: u32,
    pub cmd: u16,
    pub bytes: Bytes,
}
impl From<Vec<u8>> for LogicMessage {
    fn from(mut value: Vec<u8>) -> Self {
        let prefix = value.drain(..6).collect::<Vec<u8>>();
        let ix = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);
        let cmd = u16::from_le_bytes([prefix[4], prefix[5]]);
        let bytes = bytes::Bytes::from(value);
        LogicMessage { ix, cmd, bytes }
    }
}

impl LogicMessage {
    pub fn to_vec(logic_message: LogicMessage) -> BytesMut {
        let mut bytes_mut = BytesMut::new();

        bytes_mut.put_u32_le(logic_message.ix);
        bytes_mut.put_u16_le(logic_message.cmd);
        bytes_mut.extend_from_slice(&logic_message.bytes);
        return bytes_mut;
    }
}

struct LogicMessageCodec;
impl Decoder for LogicMessageCodec {
    type Item = LogicMessage;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(None)
    }
}

pub enum NetMessage {
    Read(LogicMessage),
    Close,
}
