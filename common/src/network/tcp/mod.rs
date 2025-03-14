use kameo::message::Context;
use kameo::{Actor, messages};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// 定义一个主服务器 Actor 来监听 TCP 连接
#[derive(Actor)]
struct TcpServerActor {
    listener: TcpListener,
}
#[messages]
impl TcpServerActor {
    fn new(listener: TcpListener) -> Self {
        TcpServerActor { listener }
    }

    #[message]
    async fn start(&mut self) -> anyhow::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    tracing::info!("Tcp:Accepted connection from: {}", addr);
                    let actor = TcpConnectionActor { stream };
                    let actor_ref = kameo::spawn(actor);
                    // 启动一个任务读取数据并发送到 Actor
                    actor_ref.tell(Start {}); //开始处理消息
                }
                Err(e) => {
                    tracing::error!("Tcp:Failed to accept connection:{}", e)
                }
            }
        }
    }
}
// 定义一个 TCP Actor 来处理单个连接
#[derive(Actor)]
struct TcpConnectionActor {
    stream: TcpStream,
}

#[messages]
impl TcpConnectionActor {
    #[message]
    async fn start(&mut self) -> anyhow::Result<()> {
        let mut buf = vec![0; 1024];
        match self.stream.read(&mut buf).await {
            Ok(0) => {
                // 对端关闭连接
                println!("Connection closed by peer");
            }
            Ok(n) => {
                let message = TcpMessage {
                    content: String::from_utf8_lossy(&buf[..n]).to_string(),
                };
                if actor_ref.send(message).await.is_err() {
                    eprintln!("Failed to send message to actor");
                }
            }
            Err(e) => {
                eprintln!("Failed to read from TCP stream: {}", e);
            }
        }

        Ok(())
    }
}
