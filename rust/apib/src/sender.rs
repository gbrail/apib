use crate::{
    collector::{Collector, LocalCollector},
    config::Config,
    connector::{Connection, Http1Connection, Http2Connection},
    error::Error,
};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Request};
use rustls_pki_types::ServerName;
use std::{sync::Arc, time::SystemTime};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const USER_AGENT: &str = "apib";

pub struct Sender<C> {
    config: Arc<Config>,
    connection: C,
    request: Option<Request<Full<Bytes>>>,
    verbose: bool,
}

impl<C> Sender<C>
where
    C: Connection,
{
    pub fn new(config: Arc<Config>, connection: C) -> Self {
        let verbose = config.verbose;
        Self {
            config,
            connection,
            request: None,
            verbose,
        }
    }

    pub async fn send(&mut self) -> Result<bool, Error> {
        let mut connection_opened = false;
        if !self.connection.connected() {
            if self.verbose {
                println!("Connecting to {}:{}...", self.config.host, self.config.port);
            }
            let new_conn =
                TcpStream::connect((self.config.host.as_str(), self.config.port)).await?;
            new_conn.set_nodelay(true)?;
            if self.verbose {
                println!("Connected");
            }
            connection_opened = true;

            if let Some(tls_config) = &self.config.tls {
                if self.verbose {
                    println!("Connecting using TLS...");
                }
                let cfg = Arc::clone(tls_config);
                let sn = ServerName::try_from(self.config.host.as_str())
                    .expect("Invalid SNI host name")
                    .to_owned();
                let connector = TlsConnector::from(cfg);
                let tls_conn = connector.connect(sn, new_conn).await?;
                if self.verbose {
                    let (_, tls_info) = tls_conn.get_ref();
                    if let Some(ap) = tls_info.alpn_protocol() {
                        println!("ALPN protocol: {:?}", ap);
                    }
                    if let Some(pv) = tls_info.protocol_version() {
                        println!("Connected using {:?}", pv);
                    }
                    if let Some(cs) = tls_info.negotiated_cipher_suite() {
                        println!("Cipher: {:?}", cs);
                    }
                }
                self.connection.connect(tls_conn).await?
            } else {
                self.connection.connect(new_conn).await?
            }
        }

        let request = match &self.request {
            Some(r) => r.clone(),
            None => {
                let mut req_builder = Request::builder()
                    .uri(self.config.path.as_str())
                    .method(&self.config.method)
                    .header("Host", self.config.host_hdr.as_str())
                    .header("User-Agent", USER_AGENT);
                for (name, value) in &self.config.headers {
                    req_builder = req_builder.header(name, value);
                }
                let new_req = req_builder.body(Full::new(self.config.body.clone()))?;
                self.request = Some(new_req.clone());
                new_req
            }
        };
        if self.verbose {
            println!("{:?}", request);
        }

        let mut response = match self.connection.send_request(request).await {
            Ok(r) => r,
            Err(e) => {
                self.connection.disconnect();
                return Err(e);
            }
        };

        if let Some(close_header) = response.headers().get("close") {
            if close_header == "close" {
                self.connection.disconnect();
            }
        }

        if !response.status().is_success() {
            //  Don't deliberately disconnect now
            return Err(Error::Http(response.status().as_u16()));
        }
        if self.verbose {
            for (key, value) in response.headers().iter() {
                println!("{}: {}", key, value.to_str().unwrap());
            }
            let body_buf = response.body_mut().collect().await?.to_bytes();
            println!("\n{:?}", body_buf);
        } else {
            while response.frame().await.is_some() {}
        }
        Ok(connection_opened)
    }

    pub async fn do_loop(&mut self, collector: &Collector) {
        let mut local_stats = LocalCollector::new();
        loop {
            let start = SystemTime::now();
            match self.send().await {
                Ok(connection_opened) => {
                    if connection_opened {
                        Collector::connection_opened(&mut local_stats);
                    }
                    if collector.success(&mut local_stats, start, SystemTime::now(), 0, 0) {
                        break;
                    }
                }
                Err(e) => {
                    if self.verbose {
                        println!("Error: {}", e);
                    }
                    if collector.failure(&mut local_stats, e) {
                        break;
                    }
                }
            }
        }
        collector.collect(local_stats);
    }
}

/*
 * See the "connector" module for why http1 and http2 need to have separate
 * implementations, but since they do, this lets us write a generic "main" or
 * test program that dispatches to a type-specific implementation of the "Sender".
 * We do all this to avoid "Box" and "dyn" because this is a performance test
 * program.
 */
pub struct SendWrapper {
    http1_sender: Option<Sender<Http1Connection>>,
    http2_sender: Option<Sender<Http2Connection>>,
}

impl SendWrapper {
    pub fn new(config: Arc<Config>, http2: bool) -> Self {
        if http2 {
            Self {
                http1_sender: None,
                http2_sender: Some(Sender::new(config, Http2Connection::new())),
            }
        } else {
            Self {
                http1_sender: Some(Sender::new(config, Http1Connection::new())),
                http2_sender: None,
            }
        }
    }

    pub async fn send(&mut self) -> Result<bool, Error> {
        if let Some(http1) = self.http1_sender.as_mut() {
            http1.send().await
        } else if let Some(http2) = self.http2_sender.as_mut() {
            http2.send().await
        } else {
            panic!("Can't get here")
        }
    }

    pub async fn do_loop(&mut self, collector: &Collector) {
        if let Some(http1) = self.http1_sender.as_mut() {
            http1.do_loop(collector).await
        } else if let Some(http2) = self.http2_sender.as_mut() {
            http2.do_loop(collector).await
        } else {
            panic!("Can't get here")
        }
    }
}
