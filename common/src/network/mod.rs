pub mod tcp;

use crate::network::tcp::session::TcpConnectionActor;
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
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync + 'static {
    async fn handle(
        &self,
        actor_ref: &ActorRef<TcpConnectionActor>,
        message: NetMessage,
    ) -> anyhow::Result<()>;
}

pub trait MessageHandlerFactory: Send + Sync + 'static + Sized {
    fn create_handler(&self) -> Box<dyn MessageHandler + Send + Sync>;
}
