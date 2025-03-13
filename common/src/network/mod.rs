use bytes::Bytes;

pub struct Msg {
    pub ix: usize,
    pub cmd: u16,
    pub bytes: Bytes,
}

pub trait Dispatcher {
    async fn frame_ready(&self, msg: Msg) -> anyhow::Result<Msg>;
}

pub struct Network {}

impl Dispatcher for Network {
    async fn frame_ready(&self, msg: Msg) -> anyhow::Result<Msg> {
        todo!()
    }
}