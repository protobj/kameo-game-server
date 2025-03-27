use crate::gate::{TcpMessageHandler, WebSocketMessageHandler};
use kameo::actor::ActorRef;
use network::LogicMessage;
use network::tcp::session::TcpSession;
use network::websocket::session::WsSession;
use prost::Message;
use prost::bytes::BytesMut;
use protocol::base_cmd::BaseError;
use std::fmt::{Debug, Display, Formatter, Write};

pub struct ClientSession {
    pub(self) ws_session_ref: Option<ActorRef<WsSession<WebSocketMessageHandler>>>,
    pub(self) tcp_session_ref: Option<ActorRef<TcpSession<TcpMessageHandler>>>,
}

impl ClientSession {
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
fn new_logic_message<T: Message>(cmd: u16, ix: u32, message: T) -> anyhow::Result<LogicMessage> {
    let mut bytes_mut = BytesMut::new();
    message
        .encode(&mut bytes_mut)
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(LogicMessage {
        cmd,
        ix,
        bytes: bytes_mut.freeze(),
    })
}

impl ClientSession {
    pub(crate) async fn handle_message(
        &mut self,
        logic_message: LogicMessage,
    ) -> anyhow::Result<Option<LogicMessage>> {
        let cmd = logic_message.cmd as i32;

        let cmd = protocol::cmd::Cmd::try_from(cmd);
        let cmd = match cmd {
            Ok(x) => x,
            Err(e) => {
                return Ok(Some(new_logic_message(
                    1,
                    logic_message.ix,
                    protocol::base_cmd::ErrorRsp {
                        code: BaseError::UnknownCommandError as i32,
                        message: BaseError::UnknownCommandError.as_str_name().to_string(),
                    },
                )?));
            }
        };

        Ok(None)
    }
}
