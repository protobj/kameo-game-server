use crate::gate::{TcpMessageHandler, WebSocketMessageHandler};
use kameo::actor::ActorRef;
use network::LogicMessage;
use network::tcp::session::TcpSession;
use network::websocket::session::WsSession;

pub struct ClientSession {
    pub(self) ws_session_ref: Option<ActorRef<WsSession<WebSocketMessageHandler>>>,
    pub(self) tcp_session_ref: Option<ActorRef<TcpSession<TcpMessageHandler>>>,
}

impl ClientSession {
    pub(crate) async fn handle_message(&mut self, logic_message: LogicMessage) {}

    pub(crate) fn new_ws(
        ws_session_ref: Option<ActorRef<WsSession<WebSocketMessageHandler>>>,
    ) -> Self {
        Self {
            ws_session_ref,
            tcp_session_ref: None,
        }
    }

    pub(crate) fn new_tcp(
        tcp_session_ref: Option<ActorRef<TcpSession<TcpMessageHandler>>>,
    ) -> Self {
        Self {
            ws_session_ref: None,
            tcp_session_ref,
        }
    }
}
