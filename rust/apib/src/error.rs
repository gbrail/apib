use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Error {
    Http(u16),
    IO(String),
    InvalidURL(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(status) => f.write_fmt(format_args!("HTTP status {}", status)),
            Error::IO(msg) => f.write_fmt(format_args!("HTTP error: {}", msg)),
            Error::InvalidURL(msg) => f.write_fmt(format_args!("Invalid URL: {}", msg)),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::InvalidURL(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<hyper::http::Error> for Error {
    fn from(err: hyper::http::Error) -> Self {
        Error::IO(err.to_string())
    }
}
