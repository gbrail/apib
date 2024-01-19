use std::{pin::Pin, sync::Arc, task::Poll};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::Collector;

pub struct CountingConnection {
    stream: TcpStream,
    collector: Arc<Collector>,
    bytes_sent: u64,
    bytes_received: u64,
}

impl CountingConnection {
    pub fn new(stream: TcpStream, collector: Arc<Collector>) -> Self {
        CountingConnection {
            stream,
            collector,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

impl AsyncRead for CountingConnection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let start = buf.filled().len();
        let s = self.get_mut();
        let stream = Pin::new(&mut s.stream);
        let result = stream.poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            let total_len = buf.filled().len() - start;
            s.bytes_received += total_len as u64;
        }
        result
    }
}

impl AsyncWrite for CountingConnection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let s = self.get_mut();
        let stream = Pin::new(&mut s.stream);
        let result = stream.poll_write(cx, buf);
        if let Poll::Ready(Ok(bytes_written)) = result {
            s.bytes_sent += bytes_written as u64;
        }
        result
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let s = self.get_mut();
        let stream = Pin::new(&mut s.stream);
        stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let s = self.get_mut();
        let stream = Pin::new(&mut s.stream);
        stream.poll_shutdown(cx)
    }
}

impl Drop for CountingConnection {
    fn drop(&mut self) {
        self.collector
            .collect_connection(self.bytes_sent, self.bytes_received);
    }
}
