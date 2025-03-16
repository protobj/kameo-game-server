use crossbeam_utils::sync::WaitGroup;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use kameo::actor::ActorRef;

pub struct LogicMessage {
    pub ix: u32,
    pub cmd: u16,
    pub bytes: Bytes,
}

pub enum NetMessage {
    Read(LogicMessage),
    Close,
}