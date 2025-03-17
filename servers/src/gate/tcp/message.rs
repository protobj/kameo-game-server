use bytes::{BufMut, Bytes, BytesMut};

#[derive(Debug)]
pub struct LogicMessage {
    pub cmd: u16,
    pub ix: u32,
    pub bytes: Bytes,
}

impl<'a> From<&'a [u8]> for LogicMessage {
    fn from(value: &'a [u8]) -> Self {
        let ix = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
        let cmd = u16::from_le_bytes([value[4], value[5]]);
        let bytes = Bytes::copy_from_slice(&value[6..]);
        LogicMessage { ix, cmd, bytes }
    }
}

impl LogicMessage {
    /// 零拷贝地生成一个 Bytes 对象，包含完整的序列化数据
    pub fn to_bytes(&self) -> Bytes {
        let mut buffer = BytesMut::with_capacity(6 + self.bytes.len());
        buffer.put_u32_le(self.ix);
        buffer.put_u16_le(self.cmd);
        buffer.extend_from_slice(&self.bytes);
        buffer.freeze()
    }
}
