use noxious::{error::ToxicUpdateError, proxy::ProxyValidateError};
use std::io;
use thiserror::Error;
use warp::http::StatusCode;

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceKind {
    Toxic,
    Proxy,
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum StoreError {
    #[error("Missing required field")]
    InvalidProxyConfig(ProxyValidateError),
    #[error("An item with this name already exists")]
    AlreadyExists,
    #[error("{0} not found")]
    NotFound(ResourceKind),
    #[error("I/O error: {0:?}")]
    IoError(io::ErrorKind),
    #[error("Proxy closed")]
    ProxyClosed,
    #[error("Internal server error")]
    Other,
}

impl From<StoreError> for StatusCode {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::InvalidProxyConfig(..) => StatusCode::BAD_REQUEST,
            StoreError::AlreadyExists => StatusCode::CONFLICT,
            StoreError::NotFound(..) => StatusCode::NOT_FOUND,
            StoreError::ProxyClosed | StoreError::IoError(..) | StoreError::Other => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        StoreError::IoError(err.kind())
    }
}

impl From<ProxyValidateError> for StoreError {
    fn from(err: ProxyValidateError) -> Self {
        StoreError::InvalidProxyConfig(err)
    }
}

impl From<ToxicUpdateError> for StoreError {
    fn from(err: ToxicUpdateError) -> Self {
        match err {
            ToxicUpdateError::NotFound => StoreError::NotFound(ResourceKind::Toxic),
            ToxicUpdateError::Other => StoreError::Other,
        }
    }
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceKind::Toxic => write!(f, "toxic"),
            ResourceKind::Proxy => write!(f, "proxy"),
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum ProxyEventError {
    #[error("{0} not found")]
    NotFound(ResourceKind),
}
