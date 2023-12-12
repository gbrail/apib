use crate::{error::Error, null_verifier::NoCertificateVerification};
use rustls::ClientConfig;
use std::sync::Arc;
use url::Url;

#[derive(Debug, Default)]
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) host_hdr: String,
    pub(crate) path: String,
    pub(crate) tls: Option<Arc<ClientConfig>>,
    pub(crate) verbose: bool,
}

impl Config {
    pub fn new(url_str: &str) -> Result<Self, Error> {
        let url = Url::parse(url_str)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(Error::IO(format!("Invalid HTTP scheme: {}", url.scheme())));
        }
        let host = match url.host_str() {
            Some(h) => h.to_string(),
            None => {
                return Err(Error::IO(format!(
                    "URL {} must have host and port",
                    url_str
                )));
            }
        };
        let port = match url.port_or_known_default() {
            Some(p) => p,
            None => {
                return Err(Error::IO(format!(
                    "URL {} must have host and port",
                    url_str
                )));
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
        })
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
}
