use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use kameo::actor::ActorRef;
use network::tcp::listener::Listener;
use network::websocket::session::WsSession;
use network::{LogicMessage, MessageHandler, SessionMessage};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pub struct WsMessageHandler;
    impl MessageHandler for WsMessageHandler {
        type Session = WsSession<Self>;

        fn message_read(
            &self,
            actor_ref: ActorRef<Self::Session>,
            logic_message: LogicMessage,
        ) -> impl Future<Output = ()> + Send {
            async move {
                actor_ref
                    .tell(SessionMessage::Write(logic_message))
                    .await
                    .expect("TODO: panic message");
            }
        }
    }

    let server_ref = kameo::spawn(Listener::new(3456, move |info, stream| async move {
        WsSession::new(info, stream, WsMessageHandler)
            .await
            .map(move |ws_session| kameo::actor::spawn(ws_session))
    }));
    server_ref.wait_startup().await;
    // tokio::signal::ctrl_c().await?;
    let request = "ws://127.0.0.1:3456";
    let ws_socket = tokio_tungstenite::connect_async(request).await?;
    let ws_socket = ws_socket.0;
    println!("Connected {}", request);

    let (mut writer, mut reader) = ws_socket.split();

    let (tx, mut rx) = mpsc::channel::<LogicMessage>(10);

    // Spawn task to read from stdin
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = stdin_reader.next_line().await {
            let message = LogicMessage {
                cmd: 10,
                ix: 20,
                bytes: Bytes::from(line),
            };
            tx.send(message).await.unwrap();
        }
    });

    // Spawn task to read from socket
    tokio::spawn(async move {
        loop {
            let message = reader.next().await.unwrap().unwrap();
            match message {
                Message::Text(_) => {}
                Message::Binary(bytes) => {
                    println!("Received: {:?}", LogicMessage::from(bytes.as_ref()));
                }
                Message::Ping(_) => {}
                Message::Pong(_) => {}
                Message::Close(_) => {}
                Message::Frame(_) => {}
            }
        }
    });

    // Handle sending messages to the server
    while let Some(msg) = rx.recv().await {
        writer.send(Message::Binary(msg.to_bytes())).await.unwrap();
    }
    tokio::signal::ctrl_c().await?;
    Ok(())
}
