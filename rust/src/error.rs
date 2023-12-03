use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Error {
    HTTPError(u16),
    IOError(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::HTTPError(status) => f.write_fmt(format_args!("HTTP status {}", status)),
            Error::IOError(msg) => f.write_fmt(format_args!("HTTP error: {}", msg)),
        }
    }
}
