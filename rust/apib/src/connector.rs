use crate::error::Error;
use async_trait::async_trait;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    client::conn::{http1, http2},
    Request, Response,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::{AsyncRead, AsyncWrite};

/*
 * A generic trait for something that can connect and send requests over various protocols.
 * In Hyper, the http1 and http2 implementations don't share a trait so we have to go
 * through all this to abstract it and still use the type system. In this program, just
 * avoiding one "Box<dyn whatever>" can save a few percent of CPU usage, so we do it.
 */
#[async_trait]
pub(crate) trait Connection {
    // Return whether "connect" was called and it worked
    fn connected(&self) -> bool;
    // Connect on some Tokio-compatible network interface
    async fn connect<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &mut self,
        conn: T,
    ) -> Result<(), Error>;
    // Drop the connection if it's open
    fn disconnect(&mut self);
    // Send a single request. Panic if "connect" did not previously succeed.
    async fn send_request(
        &mut self,
        req: Request<Full<Bytes>>,
    ) -> Result<Response<Incoming>, Error>;
}

#[derive(Default)]
pub(crate) struct Http1Connection {
    sender: Option<http1::SendRequest<Full<Bytes>>>,
}

impl Http1Connection {
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl Connection for Http1Connection {
    fn connected(&self) -> bool {
        self.sender.is_some()
    }

    fn disconnect(&mut self) {
        self.sender = None;
    }

    async fn connect<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &mut self,
        conn: T,
    ) -> Result<(), Error> {
        let io = TokioIo::new(conn);
        let (sender, conn_driver) = http1::handshake(io).await?;
        tokio::spawn(async move {
            if let Err(e) = conn_driver.await {
                println!("Error processing connection: {}", e);
            }
        });
        self.sender = Some(sender);
        Ok(())
    }

    async fn send_request(
        &mut self,
        req: Request<Full<Bytes>>,
    ) -> Result<Response<Incoming>, Error> {
        let sender = self.sender.as_mut().expect("Must connect before sending");
        Ok(sender.send_request(req).await?)
    }
}

#[derive(Default)]
pub(crate) struct Http2Connection {
    sender: Option<http2::SendRequest<Full<Bytes>>>,
}

impl Http2Connection {
    pub fn new() -> Self {
        Default::default()
    }
}

#[async_trait]
impl Connection for Http2Connection {
    fn connected(&self) -> bool {
        self.sender.is_some()
    }

    fn disconnect(&mut self) {
        self.sender = None;
    }

    async fn connect<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &mut self,
        conn: T,
    ) -> Result<(), Error> {
        let io = TokioIo::new(conn);
        let exec = TokioExecutor::new();
        let (sender, conn_driver) = http2::handshake(exec, io).await?;
        tokio::spawn(async move {
            if let Err(e) = conn_driver.await {
                println!("Error processing connection: {}", e);
            }
        });
        self.sender = Some(sender);
        Ok(())
    }

    async fn send_request(
        &mut self,
        req: Request<Full<Bytes>>,
    ) -> Result<Response<Incoming>, Error> {
        let sender = self.sender.as_mut().expect("Must connect before sending");
        Ok(sender.send_request(req).await?)
    }
}
