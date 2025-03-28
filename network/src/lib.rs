use kameo::Actor;
use kameo::actor::ActorRef;
use std::net::SocketAddr;

pub mod tcp;
pub mod websocket;

pub type NetworkPort = u16;

pub enum SessionMessage {
    Write(Package),
    Read(Package),
}

pub trait MessageHandler: Send + 'static {
    type Actor: Actor;
    fn message_read(
        &mut self,
        actor_ref: ActorRef<Self::Actor>,
        protocol: Package,
    ) -> impl Future<Output = ()> + Send;
}

pub struct NetworkStreamInfo {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

use bytes::{BufMut, Bytes, BytesMut};
//
// https://github.com/NetEase/pomelo/wiki/%E5%8D%8F%E8%AE%AE%E6%A0%BC%E5%BC%8F
#[repr(u8)]
#[derive(Debug)]
pub enum PackageType {
    Handshake = 1,     // 握手请求
    HandshakeAck = 2,  // 握手响应
    Heartbeat = 3,     // 心跳包
    Kick = 4,          // 断开连接
    Request = 5,       //请求
    Response = 6,      //响应
    ResponseError = 7, //响应错误
    Notify = 8,        //通知
    Push = 9,          //推送
}

impl PackageType {
    pub fn from_u8(typ: u8) -> Option<PackageType> {
        match typ {
            1 => Some(PackageType::Handshake),
            2 => Some(PackageType::HandshakeAck),
            3 => Some(PackageType::Heartbeat),
            4 => Some(PackageType::Kick),
            5 => Some(PackageType::Request),
            6 => Some(PackageType::Response),
            7 => Some(PackageType::ResponseError),
            8 => Some(PackageType::Notify),
            9 => Some(PackageType::Push),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Package {
    pub r#type: PackageType,
    pub data: Option<Bytes>,
}

impl Package {
    pub fn to_bytes(self) -> Bytes {
        let typ = self.r#type as u8;
        //write len
        if let Some(data) = self.data {
            let len = data.len();
            let mut buffer = BytesMut::with_capacity(6 + len);
            buffer.put_u8(typ);

            buffer.put_u8((len >> 16 & 0xFF) as u8);
            buffer.put_u8((len >> 8 & 0xFF) as u8);
            buffer.put_u8((len & 0xFF) as u8);

            buffer.put(data);
            buffer.freeze()
        } else {
            let mut buffer = BytesMut::with_capacity(1);
            buffer.put_u8(typ);
            buffer.freeze()
        }
    }
    pub fn new_control(r#type: PackageType) -> Self {
        Package { r#type, data: None }
    }
    pub fn new_data(r#type: PackageType, value: &[u8]) -> Self {
        let bytes = Bytes::copy_from_slice(value);
        Package {
            r#type,
            data: Some(bytes),
        }
    }
}
#[inline]
pub fn bytes_to_usize(len_buf: Vec<u8>) -> usize {
    let mut len = 0;
    for b in len_buf {
        len = len << 8 + b
    }
    len
}
