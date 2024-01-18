use crate::{Error, Target};

#[derive(Debug, Default)]
pub struct Builder {
    pub(crate) port: u16,
    pub(crate) use_localhost: bool,
    pub(crate) certificate: Option<String>,
    pub(crate) key: Option<String>,
    pub(crate) self_signed: bool,
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

    pub fn self_signed(mut self, ss: bool) -> Self {
        self.self_signed = ss;
        self
    }

    pub async fn build(self) -> Result<Target, Error> {
        if self.self_signed && (self.certificate.is_some() || self.key.is_some()) {
            return Err(Error::Generic(
                "Must use custom key and cert or self-signed, but not both".to_string(),
            ));
        }
        Target::new(self).await
    }
}
