use crossbeam_utils::sync::WaitGroup;
use std::pin::Pin;

pub mod tcp;

use crate::gate::tcp::session::TcpSessionActor;
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

pub type HandleFn =
    fn(ActorRef<TcpSessionActor>, NetMessage) -> Pin<Box<dyn Future<Output = ()> + Send>>;

struct GateServer {}

fn gate_main(wg: WaitGroup) {}

async fn handle(actor_ref: ActorRef<TcpSessionActor>, net_message: NetMessage) {}
