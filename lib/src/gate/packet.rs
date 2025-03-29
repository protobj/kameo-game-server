use bytes::{BufMut, Bytes, BytesMut};
use integer_encoding::VarInt;
use std::fmt::{Display, Formatter};

#[repr(u8)]
#[derive(Debug)]
pub enum Type {
    Handshake = 1,    // 握手请求
    HandshakeAck = 2, // 握手响应
    Heartbeat = 3,    // 心跳包
    Kick = 4,         // 断开连接
    Request = 5,      //请求
    Response = 6,     //响应
    Notify = 7,       //通知
    Push = 8,         //推送
}
impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Handshake => f.write_str("Handshake"),
            Type::HandshakeAck => f.write_str("HandshakeAck"),
            Type::Heartbeat => f.write_str("Heartbeat"),
            Type::Kick => f.write_str("Kick"),
            Type::Request => f.write_str("Request"),
            Type::Response => f.write_str("Response"),
            Type::Notify => f.write_str("Notify"),
            Type::Push => f.write_str("Push"),
        }
    }
}
impl Type {
    pub fn from_u8(typ: u8) -> Option<Type> {
        match typ {
            1 => Some(Type::Handshake),
            2 => Some(Type::HandshakeAck),
            3 => Some(Type::Heartbeat),
            4 => Some(Type::Kick),
            5 => Some(Type::Request),
            6 => Some(Type::Response),
            7 => Some(Type::Notify),
            8 => Some(Type::Push),
            _ => None,
        }
    }
    pub fn need_data(&self) -> bool {
        match self {
            Type::Handshake => false,
            Type::HandshakeAck => true,
            Type::Heartbeat => false,
            Type::Kick => false,
            Type::Request => true,
            Type::Response => true,
            Type::Notify => true,
            Type::Push => true,
        }
    }
}
#[derive(Debug)]
pub struct Packet {
    pub r#type: Type,
    pub cmd: i32,
    pub data: Option<Bytes>,
}

pub struct Encoder {}
impl Encoder {
    pub fn encode(packet: Packet) -> Bytes {
        let mut output = BytesMut::with_capacity(1 + packet.data.as_ref().map_or(0, |b| b.len()));
        output.put_u8(packet.r#type as u8);
        if let Some(bytes) = packet.data {
            output.put(bytes);
        }
        output.freeze()
    }
}
pub struct Decoder;
impl Decoder {
    pub fn decode(input_data: &[u8]) -> Option<Packet> {
        let package_type = Type::from_u8(input_data[0]);
        let package_type = match package_type {
            None => {
                return None;
            }
            Some(t) => t,
        };
        if package_type.need_data() {
            let cmd = i32::decode_var(input_data).unwrap().0;
            Some(Packet::new_data(
                package_type,
                cmd,
                Bytes::copy_from_slice(&input_data[1..]),
            ))
        } else {
            Some(Packet::new_control(package_type))
        }
    }
}
impl Packet {
    pub fn new_control(r#type: Type) -> Self {
        Packet {
            r#type,
            cmd: 0,
            data: None,
        }
    }
    pub fn new_data(r#type: Type, cmd: i32, bytes: Bytes) -> Self {
        Packet {
            r#type,
            cmd,
            data: Some(bytes),
        }
    }
}
