use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Error {
    IOError(String),
    TLSError(String),
    CertificateError(String),
    Generic(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(msg) => f.write_fmt(format_args!("I/O Error: {}", msg)),
            Error::IOError(msg) => f.write_fmt(format_args!("Error: {}", msg)),
            Error::CertificateError(msg) => f.write_fmt(format_args!("Certificate Error: {}", msg)),
            Error::TLSError(msg) => f.write_fmt(format_args!("TLS error: {}", msg)),
        }
    }
}

impl From<tokio::io::Error> for Error {
    fn from(e: tokio::io::Error) -> Self {
        Error::IOError(e.to_string())
    }
}

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Self {
        Error::TLSError(e.to_string())
    }
}

impl From<makecert::Error> for Error {
    fn from(e: makecert::Error) -> Self {
        Error::CertificateError(e.to_string())
    }
}
