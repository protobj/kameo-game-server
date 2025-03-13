use bytes::{Buf, BytesMut};
use prost::encoding::message;
use tokio::net::{TcpListener, TcpStream};

async fn start_gate_tcp_server(host: &str, port: u32) -> anyhow::Result<()> {
    // 监听本地 8080 端口
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    tracing::info!("Server listening on {}:{}", host, port);

    loop {
        // 接受新的连接
        let (socket, addr) = listener.accept().await?;
        tracing::info!("New client connected: {}", addr);

        // 为每个连接生成异步任务
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                tracing::error!("Client {} error: {}", addr, e);
            }
        });
    }
}

async fn handle_client(mut socket: TcpStream) -> anyhow::Result<()> {
    // 使用 Tokio 的异步读写工具
    let (mut reader, mut writer) = socket.split();

    // 缓冲区存储接收到的字节
    let mut buf = BytesMut::new();

    loop {
        // 从客户端读取数据
        match reader.readable().await {
            Ok(_) => {
                // 读取字节到缓冲区
                let read_bytes = reader.try_read_buf(&mut buf)?;
                if read_bytes == 0 {
                    // 客户端关闭连接
                    return Ok(());
                }

                // 解析 Protobuf 消息 (带长度前缀)
                let request = parse_protobuf(&mut buf)?;
                tracing::info!("Received request: {:?}", request);

                // 处理请求并生成响应
                let response = process_request(request);

                // 编码响应并发送回客户端
                send_response(&mut writer, response).await?;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

// 解析带长度前缀的 Protobuf 消息
fn parse_protobuf(buf: &mut BytesMut) -> anyhow::Result<()> {
    // 读取长度前缀 (假设是 4 字节大端序)
    if buf.len() < 4 {
        return Err("Incomplete length prefix".into());
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    buf.advance(4); // 跳过长度前缀

    // 检查是否收到完整消息
    if buf.len() < len {
        return Err("Incomplete message".into());
    }

    // 解析消息体
    let body = buf.split_to(len);
    let request = message::Request::decode(body)?;

    Ok(request)
}

// 处理请求的逻辑
fn process_request(request: message::Request) -> message::Response {
    message::Response {
        result: format!("Processed: {}", request.query),
    }
}

// 编码并发送响应 (带长度前缀)
async fn send_response(
    writer: &mut tokio::net::tcp::WriteHalf<'_>,
    response: message::Response,
) -> Result<(), Box<dyn Error>> {
    // 编码消息体
    let mut body = BytesMut::new();
    response.encode(&mut body)?;

    // 添加长度前缀 (4 字节大端序)
    let len = body.len() as u32;
    let mut frame = BytesMut::with_capacity(4 + body.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&body);

    // 发送数据
    writer.write_all(&frame).await?;
    Ok(())
}
