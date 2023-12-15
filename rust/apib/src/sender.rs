use crate::{
    collector::{Collector, LocalCollector},
    config::{Config, HttpMode},
    error::Error,
};
use async_trait::async_trait;
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Body, Bytes, Incoming},
    client::conn::{http1, http2},
    Request, Response,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls_pki_types::ServerName;
use std::{sync::Arc, time::SystemTime};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;

//const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "apib";

/*
 * This next complexity is necessary because Hyper doesn't have a common
 * trait between HTTP/1 and HTTP/2 senders, and we don't want to go totally
 * trait-crazy just yet. Maybe object-oriented programming wasn't the evil
 * thing that the cool kids say today.
 */
#[async_trait]
trait Holder<B> {
    async fn send_request(&mut self, req: Request<B>) -> hyper::Result<Response<Incoming>>;
}

struct Http1Holder<B> {
    sender: http1::SendRequest<B>,
}

impl<B> Http1Holder<B> {
    fn new(sender: http1::SendRequest<B>) -> Self {
        Http1Holder { sender }
    }
}

#[async_trait]
impl<B> Holder<B> for Http1Holder<B>
where
    B: Body + Send + 'static,
{
    async fn send_request(&mut self, req: Request<B>) -> hyper::Result<Response<Incoming>> {
        self.sender.send_request(req).await
    }
}

struct Http2Holder<B> {
    sender: http2::SendRequest<B>,
}

impl<B> Http2Holder<B> {
    fn new(sender: http2::SendRequest<B>) -> Self {
        Http2Holder { sender }
    }
}

#[async_trait]
impl<B> Holder<B> for Http2Holder<B>
where
    B: Body + Send + 'static,
{
    async fn send_request(&mut self, req: Request<B>) -> hyper::Result<Response<Incoming>> {
        self.sender.send_request(req).await
    }
}

pub struct Sender {
    config: Arc<Config>,
    sender: Option<Box<dyn Holder<Full<Bytes>> + Send + Sync>>,
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
        &self,
        conn: T,
    ) -> Result<Box<dyn Holder<Full<Bytes>> + Send + Sync>, Error> {
        let io = TokioIo::new(conn);
        match self.config.http_mode {
            HttpMode::Http1 => {
                let (sender, conn_driver) = http1::handshake(io).await?;
                tokio::spawn(async move {
                    if let Err(e) = conn_driver.await {
                        println!("Error processing connection: {}", e);
                    }
                });
                Ok(Box::new(Http1Holder::new(sender)))
            }
            HttpMode::Http2 => {
                let (sender, conn_driver) = http2::handshake(TokioExecutor::new(), io).await?;
                tokio::spawn(async move {
                    if let Err(e) = conn_driver.await {
                        println!("Error processing connection: {}", e);
                    }
                });
                Ok(Box::new(Http2Holder::new(sender)))
            }
        }
    }

    pub async fn send(&mut self) -> Result<bool, Error> {
        let mut connection_opened = false;
        let mut holder = if self.sender.is_none() {
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
                self.start_connection(tls_conn).await?
            } else {
                self.start_connection(new_conn).await?
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

        let mut response = holder.send_request(request).await?;

        let should_close = if let Some(close_header) = response.headers().get("close") {
            close_header == "close"
        } else {
            false
        };

        if !response.status().is_success() {
            // We can re-use the connection now
            if !should_close {
                self.sender = Some(holder);
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
            self.sender = Some(holder);
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
