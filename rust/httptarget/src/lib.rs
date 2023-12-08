mod error;
mod service;

pub use error::Error;

use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::{net::TcpListener, sync::oneshot};

pub struct Target {
    address: SocketAddr,
    stop_channel: Option<oneshot::Sender<()>>,
}

impl Target {
    pub async fn new(port: u16, use_localhost: bool) -> Result<Self, Error> {
        let addr = if use_localhost {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
        } else {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
        };
        let listener = TcpListener::bind(addr).await?;
        let actual_addr = listener.local_addr().expect("Error getting my address");
        let (sender, mut receiver) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _)) => {
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
}

impl Drop for Target {
    fn drop(&mut self) {
        self.stop();
    }
}
