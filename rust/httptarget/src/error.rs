use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Error {
    IOError(String),
    Generic(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(msg) => f.write_fmt(format_args!("Error: {}", msg)),
            Error::IOError(msg) => f.write_fmt(format_args!("Error: {}", msg)),
        }
    }
}

impl From<tokio::io::Error> for Error {
    fn from(e: tokio::io::Error) -> Self {
        Error::IOError(e.to_string())
    }
}
