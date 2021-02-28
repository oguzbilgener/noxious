use noxious::proxy::ProxyValidateError;
use std::io;
use thiserror::Error;
use warp::http::StatusCode;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum StoreError {
    #[error("Missing required field")]
    InvalidProxyConfig(ProxyValidateError),
    #[error("An item with this name already exists")]
    AlreadyExists,
    #[error("Item not found")]
    NotFound,
    #[error("I/O error: {0:?}")]
    IoError(io::ErrorKind),
    #[error("Internal server error")]
    Other,
}

impl From<StoreError> for StatusCode {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::InvalidProxyConfig(..) => StatusCode::BAD_REQUEST,
            StoreError::AlreadyExists => StatusCode::CONFLICT,
            StoreError::NotFound => StatusCode::NOT_FOUND,
            StoreError::IoError(..) | StoreError::Other => StatusCode::INTERNAL_SERVER_ERROR,
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
