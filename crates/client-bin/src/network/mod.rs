use std::fmt;
use std::fmt::{Debug, Formatter};
use rustls::{Error};

pub mod tls_client;
pub mod tls_config;

pub enum NetworkError {
    InvalidAddress,
    TlsError(rustls::Error),
    IoError(std::io::Error),
}

impl From<rustls::Error> for NetworkError {
    fn from(value: Error) -> Self {
        NetworkError::TlsError(value)
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(value: std::io::Error) -> Self {
        NetworkError::IoError(value)
    }
}

impl Debug for NetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::InvalidAddress => write!(f, "Invalid Address"),
            NetworkError::TlsError(e) => write!(f, "TLS Error: {:?}", e),
            NetworkError::IoError(e) => write!(f, "IO Error: {:?}", e),
        }
    }
}
