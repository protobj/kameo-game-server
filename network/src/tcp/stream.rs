use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// A network port
pub type NetworkPort = u16;

/// Incoming encryption mode
pub struct NetworkStream {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub stream: TcpStream,
}

pub struct NetworkStreamInfo {
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

impl NetworkStream {
    pub fn info(&self) -> NetworkStreamInfo {
        NetworkStreamInfo {
            peer_addr: self.peer_addr(),
            local_addr: self.local_addr(),
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn into_split(self) -> (ReaderHalf, WriterHalf) {
        let (read, write) = self.stream.into_split();
        (ReaderHalf(read), WriterHalf(write))
    }
}

pub struct ReaderHalf(OwnedReadHalf);
impl ReaderHalf {
    pub async fn read_n_bytes(&mut self, len: usize) -> Result<Vec<u8>, tokio::io::Error> {
        let mut buf = vec![0u8; len];
        let mut c_len = 0;
        let stream = &mut self.0;
        stream.readable().await?;

        while c_len < len {
            let n = self.read(buf.as_mut_slice()).await?;
            if n == 0 {
                // EOF
                return Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::UnexpectedEof,
                    "EOF",
                ));
            }
            c_len += n;
        }
        Ok(buf)
    }

    pub async fn read_u32(&mut self) -> tokio::io::Result<u32> {
        self.0.read_u32().await
    }

    pub async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> tokio::io::Result<usize> {
        self.0.read(buf).await
    }
}

impl AsyncRead for ReaderHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll_read(cx, buf)
    }
}

// =========================== WriterHalf =========================== //

pub struct  WriterHalf(pub OwnedWriteHalf);

impl WriterHalf {
    pub async fn write_u32(&mut self, n: u32) -> tokio::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        self.0.write_u32(n).await
    }
    pub async fn write_all(&mut self, data: &[u8]) -> tokio::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        self.0.write_all(data).await
    }

    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        self.0.flush().await
    }
}
