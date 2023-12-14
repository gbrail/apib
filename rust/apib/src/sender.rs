use crate::{
    collector::{Collector, LocalCollector},
    config::Config,
    error::Error,
};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::Bytes,
    client::conn::http1::{self, SendRequest},
    Request,
};
use hyper_util::rt::TokioIo;
use rustls_pki_types::ServerName;
use std::{sync::Arc, time::SystemTime};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;

//const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "apib";

pub struct Sender {
    config: Arc<Config>,
    sender: Option<SendRequest<Full<Bytes>>>,
    request: Option<Request<Full<Bytes>>>,
    verbose: bool,
}

impl Sender {
    pub fn new(config: Arc<Config>) -> Self {
        let verbose = config.verbose;
        Self {
            config,
            sender: None,
            request: None,
            verbose,
        }
    }

    async fn start_connection<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        conn: T,
    ) -> Result<SendRequest<Full<Bytes>>, Error> {
        let io = TokioIo::new(conn);
        let (sender, conn_driver) = http1::handshake(io).await?;
        tokio::spawn(async move {
            if let Err(e) = conn_driver.await {
                println!("Error processing connection: {}", e);
            }
        });
        Ok(sender)
    }

    pub async fn send(&mut self) -> Result<bool, Error> {
        let mut connection_opened = false;
        let mut sender = if self.sender.is_none() {
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
                Self::start_connection(tls_conn).await?
            } else {
                Self::start_connection(new_conn).await?
            }
        } else {
            // Take the sender saved on this object -- we'll put it back on success.
            self.sender.take().unwrap()
        };

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

        let mut response = sender.send_request(request).await?;

        let should_close = if let Some(close_header) = response.headers().get("close") {
            close_header == "close"
        } else {
            false
        };

        if !response.status().is_success() {
            // We can re-use the connection now
            if !should_close {
                self.sender = Some(sender);
            }
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
        if !should_close {
            self.sender = Some(sender);
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
