use bytes::{Buf, BytesMut};
use fastwebsockets::{Frame, OpCode, Payload, Role, WebSocket, WebSocketError};
use kameo::actor::ActorRef;
use network::tcp::listener::Listener;
use network::websocket::session::WsSession;
use network::{MessageHandler, Package, PackageType, SessionMessage};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pub struct WsMessageHandler;
    impl MessageHandler for WsMessageHandler {
        type Actor = WsSession<Self>;

        fn message_read(
            &mut self,
            actor_ref: ActorRef<Self::Actor>,
            logic_message: Package,
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
    let stream = TcpStream::connect(request).await?;

    let socket = WebSocket::after_handshake(stream, Role::Client);
    println!("Connected {}", request);

    let (mut reader, mut writer) = socket.split(tokio::io::split);

    let (tx, mut rx) = mpsc::channel::<Package>(10);

    // Spawn task to read from stdin
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = stdin_reader.next_line().await {
            tx.send(Package::new_control(PackageType::Handshake))
                .await
                .unwrap();
        }
    });

    // Spawn task to read from socket
    tokio::spawn(async move {
        loop {
            let result = reader
                .read_frame::<_, WebSocketError>(&mut move |_| async {
                    // match frame.opcode {
                    //     OpCode::Continuation => {}
                    //     OpCode::Text => {}
                    //     OpCode::Binary => {}
                    //     OpCode::Close => {}
                    //     OpCode::Ping => {}
                    //     OpCode::Pong => {}
                    // }
                    unreachable!();
                })
                .await;
            match result {
                Ok(mut frame) => match frame.opcode {
                    OpCode::Continuation => {}
                    OpCode::Text => {}
                    OpCode::Binary => {
                        let x = frame.payload.to_mut();
                        let mut bytes = BytesMut::with_capacity(x.len());
                        bytes.copy_from_slice(x);

                        let typ = bytes.get_u8();
                        let typ = PackageType::from_u8(typ).unwrap();
                        let pack = if bytes.len() > 0 {
                            Package::new_data(typ, bytes.as_ref())
                        } else {
                            Package::new_control(typ)
                        };

                        println!("Received: {:?}", pack);
                    }
                    OpCode::Close => {}
                    OpCode::Ping => {}
                    OpCode::Pong => {}
                },
                Err(_) => {}
            }
        }
    });

    // Handle sending messages to the server
    while let Some(msg) = rx.recv().await {
        writer
            .write_frame(Frame::binary(Payload::Borrowed(msg.to_bytes().as_ref())))
            .await?;
    }
    tokio::signal::ctrl_c().await?;
    Ok(())
}
