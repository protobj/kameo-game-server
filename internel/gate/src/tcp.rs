use anyhow::anyhow;
use bytes::{BufMut, BytesMut};
use ractor::{Actor, ActorProcessingErr, ActorRef, ActorRuntime};
use ractor_actors::net::tcp::{
    Frame, FrameReceiver, IncomingEncryptionMode, Listener, ListenerMessage, ListenerStartupArgs,
    NetworkStream, SessionAcceptor, TcpSession, TcpSessionMessage, TcpSessionStartupArguments,
};
use std::collections::HashMap;
use std::sync::Arc;

async fn listen_tcp() -> anyhow::Result<()> {
    //tcp监听
    let listener: Listener<Arc<TcpSessionAcceptor>> = Listener::default();
    let acceptor = Arc::new(TcpSessionAcceptor { tcp_server: None });
    let start_up_args = ListenerStartupArgs {
        port: 9090,
        encryption: IncomingEncryptionMode::Raw,
        acceptor: acceptor.clone(),
    };
    let (listener, _) = Actor::spawn(Some("TcpListener".to_string()), listener, start_up_args)?;
    acceptor.tcp_server = Some(listener);
    Ok(())
}

struct TcpSessionAcceptor {
    tcp_server: Option<ActorRef<ListenerMessage>>,
}
#[ractor::async_trait]
impl SessionAcceptor for TcpSessionAcceptor {
    async fn new_session(&self, session: NetworkStream) -> anyhow::Result<()> {
        //将自定义session跟自定义server绑定
        let local_addr = session.local_addr();
        let peer_addr = session.peer_addr();
        tracing::info!("TCP session starting,local_addr:{local_addr}, peer_addr:{peer_addr}");
        let tcp_session = TcpSession::new();
        let receiver = Arc::new(GateSessionReceiver {
            byte_buffer: BytesMut::new(),
            session: None,
        });
        let session_actor = Actor::spawn(
            None,
            tcp_session,
            TcpSessionStartupArguments {
                receiver: receiver.clone(),
                tcp_session: session,
            },
        )?;
        receiver.session = Some(session_actor);
        Ok(())
    }
}
pub struct GateSessionReceiver {
    pub byte_buffer: BytesMut,
    pub session: Some(ActorRef<TcpSessionMessage>),
}
impl FrameReceiver for GateSessionReceiver {
    async fn frame_ready(&mut self, f: Frame) -> Result<(), ActorProcessingErr> {
        // self.session.send_message()B
        self.byte_buffer.put_slice(&f);
        //开始读取数据
        if let Some(actor) = self.session {
            
        }

        Ok(())
    }
}
