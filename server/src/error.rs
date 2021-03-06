use noxious::{error::ToxicUpdateError, proxy::ProxyValidateError};
use serde::Serialize;
use std::io;
use thiserror::Error;
use warp::{
    http::StatusCode,
    reply::{json as json_reply, with_status, Response},
    Reply,
};

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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApiErrorResponse {
    #[serde(rename = "error")]
    message: String,
    #[serde(rename = "status")]
    status_code: u16,
    #[serde(skip)]
    pub code: StatusCode,
}

impl From<StoreError> for StatusCode {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::InvalidProxyConfig(..) => StatusCode::BAD_REQUEST,
            StoreError::AlreadyExists => StatusCode::CONFLICT,
            StoreError::NotFound(..) => StatusCode::NOT_FOUND,
            StoreError::ProxyClosed | StoreError::IoError(..) | StoreError::Other => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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

impl From<StoreError> for ApiErrorResponse {
    fn from(err: StoreError) -> Self {
        let message = err.to_string();
        let code: StatusCode = err.into();
        ApiErrorResponse {
            message,
            status_code: code.as_u16(),
            code,
        }
    }
}

impl ApiErrorResponse {
    pub fn new(message: &str, code: StatusCode) -> Self {
        ApiErrorResponse {
            message: message.to_owned(),
            status_code: code.as_u16(),
            code,
        }
    }
}

impl From<ApiErrorResponse> for Response {
    fn from(resp: ApiErrorResponse) -> Self {
        with_status(json_reply(&resp), resp.code).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_validate_error_into_store_error() {
        let err: StoreError = ProxyValidateError::MissingName.into();
        assert_eq!(
            StoreError::InvalidProxyConfig(ProxyValidateError::MissingName),
            err
        );
    }

    #[test]
    fn toxic_update_error_into_store_error() {
        let err: StoreError = ToxicUpdateError::NotFound.into();
        assert_eq!(StoreError::NotFound(ResourceKind::Toxic), err);

        let err: StoreError = ToxicUpdateError::Other.into();
        assert_eq!(StoreError::Other, err);
    }

    #[test]
    fn io_error_into_store_error() {
        let err: StoreError = io::Error::new(io::ErrorKind::AddrInUse, ":(").into();
        assert_eq!(StoreError::IoError(io::ErrorKind::AddrInUse), err);
    }

    #[test]
    fn other_server_error_into_status_code() {
        let code: StatusCode = StoreError::Other.into();
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, code);

        let code: StatusCode = StoreError::ProxyClosed.into();
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, code);

        let err: StoreError = io::Error::new(io::ErrorKind::AddrInUse, ":(").into();
        let code: StatusCode = err.into();
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, code);
    }
}
