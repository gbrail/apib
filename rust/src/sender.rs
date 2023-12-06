use crate::collector::{Collector, LocalCollector};
use crate::error::Error;
use crate::tokio_rt::TokioIo;
use http_body_util::BodyExt;
use hyper::client::conn::http1::{self, SendRequest};
use hyper::Request;
use rustls::ClientConfig;
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use url::Url;

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "apib";

pub struct Sender {
    host: String,
    port: u16,
    host_hdr: String,
    path: String,
    tls: Option<Arc<ClientConfig>>,
    verbose: bool,
    sender: Option<SendRequest<String>>,
}

impl Sender {
    pub fn new(u: &str) -> Result<Self, Error> {
        let url = Url::parse(u)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(Error::IOError(format!(
                "Invalid HTTP scheme: {}",
                url.scheme()
            )));
        }
        let host = match url.host_str() {
            Some(h) => h.to_string(),
            None => {
                return Err(Error::IOError(format!("URL {} must have host and port", u)));
            }
        };
        let port = match url.port_or_known_default() {
            Some(p) => p,
            None => {
                return Err(Error::IOError(format!("URL {} must have host and port", u)));
            }
        };
        let host_hdr = match url.port() {
            Some(p) => format!("{}:{}", host, p),
            None => host.clone(),
        };
        let path = match url.query() {
            Some(q) => format!("{}?{}", url.path(), q),
            None => url.path().to_string(),
        };
        let tls = if url.scheme() == "https" {
            let cfg = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {}))
                .with_no_client_auth();
            Some(Arc::new(cfg))
        } else {
            None
        };

        Ok(Self {
            host,
            port,
            host_hdr,
            path,
            tls,
            verbose: false,
            sender: None,
        })
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub async fn send(&mut self) -> Result<(), Error> {
        let mut sender = if self.sender.is_none() {
            if self.verbose {
                println!("Connecting to {}:{}...", self.host, self.port);
            }
            let new_conn = TcpStream::connect((self.host.as_str(), self.port)).await?;
            new_conn.set_nodelay(true)?;
            if self.verbose {
                println!("Connected");
            }

            let (sender, conn_driver) = if let Some(tls_config) = &self.tls {
                if self.verbose {
                    println!("Connecting using TLS...");
                }
                let cfg = Arc::clone(tls_config);
                let sn = ServerName::try_from(self.host.as_str())
                    .expect("Invalid SNI host name")
                    .to_owned();
                let connector = TlsConnector::from(cfg);
                let tls_conn = connector.connect(sn, new_conn).await?;
                let io = TokioIo::new(tls_conn);
                http1::handshake(io).await?
            } else {
                let io = TokioIo::new(new_conn);
                http1::handshake(io).await?
            };

            let (sender, conn_driver) = ;
            tokio::spawn(async move {
                if let Err(e) = conn_driver.await {
                    println!("Error processing connection: {}", e);
                }
            });
            sender
        } else {
            self.sender.take().unwrap()
        };

        let request = Request::builder()
            .uri(self.path.as_str())
            .header("Host", self.host_hdr.as_str())
            .header("User-Agent", USER_AGENT)
            .body("".to_string())?;

        if self.verbose {
            println!("{:?}", request);
        }

        let mut response = sender.send_request(request).await?;

        if !response.status().is_success() {
            // We can re-use the connection now
            self.sender = Some(sender);
            return Err(Error::HTTPError(response.status().as_u16()));
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
        self.sender = Some(sender);
        Ok(())
    }

    pub async fn do_loop(&mut self, collector: &Collector) {
        let mut local_stats = LocalCollector::new();
        loop {
            let start = SystemTime::now();
            match self.send().await {
                Ok(_) => {
                    local_stats.success(start, 0, 0);
                    if collector.success() {
                        break;
                    }
                }
                Err(e) => {
                    if self.verbose {
                        println!("Error: {}", e);
                    }
                    local_stats.failure();
                    if collector.failure(e) {
                        break;
                    }
                }
            }
        }
        collector.collect(local_stats);
    }
}

#[derive(Debug)]
struct NoCertificateVerification {}
impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![]
    }
}
