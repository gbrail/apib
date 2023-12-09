mod builder;
mod error;
mod service;
mod tls;

pub use builder::Builder;
pub use error::Error;

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use rustls::ServerConfig;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tls::make_server_config;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tokio_rustls::TlsAcceptor;

pub struct Target {
    address: SocketAddr,
    stop_channel: Option<oneshot::Sender<()>>,
}

impl Target {
    pub(crate) async fn new(builder: Builder) -> Result<Self, Error> {
        let addr = if builder.use_localhost {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), builder.port)
        } else {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), builder.port)
        };
        let listener = TcpListener::bind(addr).await?;
        let actual_addr = listener.local_addr().expect("Error getting my address");
        let (sender, mut receiver) = oneshot::channel();

        let tls_config = if builder.certificate.is_some() && builder.key.is_some() {
            Some(Arc::new(make_server_config(&builder)?))
        } else {
            None
        };

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _)) => {
                                match tls_config.as_ref() {
                                    Some(cfg) => {
                                        if let Err(e) = Self::run_tls(stream, cfg).await {
                                            println!("Error accepting TLS: {}", e);
                                        }
                                    }
                                    None => Self::run_plain(stream),
                                }
                            }
                            Err(e) => {
                                println!("Error on accept: {}", e);
                            }
                        }
                    }
                    _ = &mut receiver => {
                        break;
                    }
                };
            }
        });

        Ok(Target {
            address: actual_addr,
            stop_channel: Some(sender),
        })
    }

    pub fn stop(&mut self) {
        if let Some(c) = self.stop_channel.take() {
            c.send(()).expect("Error on send");
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    fn run_plain(stream: TcpStream) {
        let io = TokioIo::new(stream);
        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service_fn(service::handle))
                .await
            {
                println!("Error on connection: {}", e)
            }
        });
    }

    async fn run_tls(stream: TcpStream, tls_cfg: &Arc<ServerConfig>) -> Result<(), Error> {
        let cfg = Arc::clone(tls_cfg);
        let acceptor = TlsAcceptor::from(cfg);
        let tls_stream = acceptor.accept(stream).await?;
        let io = TokioIo::new(tls_stream);
        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service_fn(service::handle))
                .await
            {
                println!("Error on connection: {}", e)
            }
        });
        Ok(())
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        self.stop();
    }
}
