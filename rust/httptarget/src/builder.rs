use crate::{Error, Target};

#[derive(Debug, Default)]
pub struct Builder {
    pub(crate) port: u16,
    pub(crate) use_localhost: bool,
    pub(crate) certificate: Option<String>,
    pub(crate) key: Option<String>,
}

impl Builder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn use_localhost(mut self, lh: bool) -> Self {
        self.use_localhost = lh;
        self
    }

    pub fn certificate(mut self, cert: &str) -> Self {
        self.certificate = Some(cert.to_string());
        self
    }

    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    pub async fn build(self) -> Result<Target, Error> {
        Target::new(self).await
    }
}
