use std::fmt::{Display, Formatter};

use openssl::error::ErrorStack;

#[derive(Clone, Debug)]
pub enum Error {
    Generic(String),
    OpenSSL(ErrorStack),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(msg) => f.write_fmt(format_args!("Error: {}", msg)),
            Error::OpenSSL(stack) => f.write_fmt(format_args!("OpenSSL Error: {}", stack)),
        }
    }
}

impl From<ErrorStack> for Error {
    fn from(value: openssl::error::ErrorStack) -> Self {
        Error::OpenSSL(value)
    }
}
