use bytes::Bytes;
use kameo::actor;
use kameo::actor::ActorRef;
// use kameo::remote::ActorSwarm;
// use kameo::remote::dial_opts::DialOpts;
use servers::gate::tcp::session::Session;
use std::{pin::Pin, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;
use network::tcp::listener::Listener;
use network::tcp::message::LogicMessage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug".parse::<EnvFilter>()?)
        .without_time()
        .with_target(false)
        .init();

    // let actor_swarm =
    //     ActorSwarm::bootstrap().map_err(|e| anyhow::anyhow!("failed to bootstrap:{}", e))?;
    // actor_swarm
    //     .dial(
    //         DialOpts::unknown_peer_id()
    //             .address("/ip4/127.0.0.1/udp/8020/quic-v1".parse()?)
    //             .build(),
    //     )
    //     .await?;
    // let actor = NetServerActor::new();

    // let actor_ref = actor::spawn(actor);

    // tokio::time::sleep(Duration::from_secs(1)).await;
    // tokio::spawn(async {

    // });

    let server_ref = kameo::spawn(Listener::new(
        3456,
        move |stream| async move { Ok(kameo::spawn(Session::new(stream))) },
    ));
    server_ref.wait_startup().await;
    // tokio::signal::ctrl_c().await?;
    let stream = TcpStream::connect("127.0.0.1:3456").await?;

    tracing::info!(
        "Connected peer={}, local={}",
        stream.peer_addr()?,
        stream.local_addr()?
    );

    let (reader, mut writer) = stream.into_split();

    let mut reader = BufReader::new(reader);
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
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).await.is_err() {
                eprintln!("Connection closed by server.");
                break;
            }

            let msg_len = u32::from_be_bytes(len_buf) as usize;
            let mut buffer = vec![0u8; msg_len];

            if reader.read_exact(&mut buffer).await.is_err() {
                eprintln!("Failed to read message.");
                break;
            }
            println!("Received: {:?}", LogicMessage::from(buffer.as_slice()));
        }
    });

    // Handle sending messages to the server
    while let Some(msg) = rx.recv().await {
        let msg_bytes = msg.to_bytes();
        let msg_len = msg_bytes.len() as u32;
        let mut packet = Vec::with_capacity(4 + msg_bytes.len());
        packet.extend_from_slice(&msg_len.to_be_bytes());
        packet.extend_from_slice(&msg_bytes);
        eprintln!(
            "Sending payload of {} bytes. Network byte count {}",
            msg_len,
            packet.len()
        );
        if writer.write_all(&packet).await.is_err() {
            eprintln!("Failed to send message.");
            break;
        }
    }
    tokio::signal::ctrl_c().await?;
    Ok(())
}
