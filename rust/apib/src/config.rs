use crate::{error::Error, null_verifier::NoCertificateVerification};
use hyper::{body::Bytes, Method};
use rustls::ClientConfig;
use std::{str::FromStr, sync::Arc};
use tokio::fs;
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) host_hdr: String,
    pub(crate) path: String,
    pub(crate) method: Method,
    pub(crate) body: Bytes,
    pub(crate) tls: Option<Arc<ClientConfig>>,
    pub(crate) verbose: bool,
}

#[derive(Debug, Default)]
pub struct Builder {
    url: Option<String>,
    method: Option<String>,
    body_text: Option<String>,
    body_file: Option<String>,
    verbose: bool,
}

/*
 * TODO:
 * * Headers (support multiple instances)
 * * Warm-up time
 * * TLS verification on and off
 * * CSV output and title
 * * More output data
 * * Number formatting on output
 * * HTTP/2 on and off in various ways
 * * HTTP/2 streaming options
 */

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn set_method(mut self, method: &str) -> Self {
        self.method = Some(method.to_string());
        self
    }

    pub fn set_body_text(mut self, text: &str) -> Self {
        self.body_text = Some(text.to_string());
        self
    }

    pub fn set_body_file(mut self, f: &str) -> Self {
        self.body_file = Some(f.to_string());
        self
    }

    pub fn set_verbose(mut self, enabled: bool) -> Self {
        self.verbose = enabled;
        self
    }

    pub async fn build(self) -> Result<Config, Error> {
        if self.url.is_none() {
            return Err(Error::Configuration("URL must be set".to_string()));
        }

        let url_str = self.url.unwrap();
        let url = Url::parse(&url_str)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(Error::IO(format!("Invalid HTTP scheme: {}", url.scheme())));
        }
        let host = match url.host_str() {
            Some(h) => h.to_string(),
            None => {
                return Err(Error::Configuration(format!(
                    "URL {} must have host and port",
                    url_str
                )));
            }
        };
        let port = match url.port_or_known_default() {
            Some(p) => p,
            None => {
                return Err(Error::Configuration(format!(
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
        let body = match &self.body_file {
            Some(bf) => Self::read_body_file(bf).await?,
            None => match self.body_text {
                Some(t) => Bytes::from(t.to_string()),
                None => Bytes::new(),
            },
        };
        let method = match self.method {
            Some(method) => match Method::from_str(&method) {
                Ok(m) => m,
                Err(_) => {
                    return Err(Error::Configuration(format!(
                        "Invalid HTTP method {}",
                        method
                    )))
                }
            },
            None => {
                if body.is_empty() {
                    Method::GET
                } else {
                    Method::POST
                }
            }
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

        Ok(Config {
            host,
            port,
            host_hdr,
            path,
            method,
            body,
            tls,
            verbose: self.verbose,
        })
    }

    async fn read_body_file(file_name: &str) -> Result<Bytes, Error> {
        let file_bytes = fs::read(file_name).await?;
        Ok(Bytes::from(file_bytes))
    }
}
