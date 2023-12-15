use crate::{error::Error, null_verifier::NoCertificateVerification};
use hyper::{body::Bytes, Method};
use rustls::ClientConfig;
use std::{str::FromStr, sync::Arc};
use tokio::fs;
use url::Url;

#[derive(Debug, Default, PartialEq)]
pub(crate) enum HttpMode {
    #[default]
    Http1,
    Http2,
}

#[derive(Debug)]
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) host_hdr: String,
    pub(crate) path: String,
    pub(crate) method: Method,
    pub(crate) body: Bytes,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) tls: Option<Arc<ClientConfig>>,
    pub(crate) verbose: bool,
}

#[derive(Debug, Default)]
pub struct Builder {
    url: Option<String>,
    method: Option<String>,
    body_text: Option<String>,
    body_file: Option<String>,
    headers: Vec<(String, String)>,
    tls_no_verify: bool,
    http_mode: HttpMode,
    verbose: bool,
}

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

    pub fn add_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn set_tls_no_verify(mut self, nv: bool) -> Self {
        self.tls_no_verify = nv;
        self
    }

    pub fn set_http2(mut self, http2: bool) -> Self {
        self.http_mode = if http2 {
            HttpMode::Http2
        } else {
            HttpMode::Http1
        };
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
            let cfg = if self.tls_no_verify {
                rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {}))
                    .with_no_client_auth()
            } else {
                let mut root_store = rustls::RootCertStore::empty();
                for cert in
                    rustls_native_certs::load_native_certs().expect("could not load platform certs")
                {
                    root_store.add(cert).expect("Error loading native cert");
                }
                rustls::ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth()
            };
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
            headers: self.headers,
            tls,
            verbose: self.verbose,
        })
    }

    async fn read_body_file(file_name: &str) -> Result<Bytes, Error> {
        let file_bytes = fs::read(file_name).await?;
        Ok(Bytes::from(file_bytes))
    }
}
