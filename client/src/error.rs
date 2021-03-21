use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The errors that the client returns
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ClientError {
    /// An I/O error happened
    #[error("I/O error: {0:?}")]
    IoError(String),
    /// Server returned an error
    #[error("API error: {0}")]
    ApiError(ApiErrorResponse),
    /// Unexpected status code, cannot parse the response body
    #[error("Unexpected response code {0}, expected {1}")]
    UnexpectedStatusCode(u16, u16),
}

/// The struct that describes the error response
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiErrorResponse {
    /// The error message
    #[serde(rename = "error")]
    pub message: String,
    /// The error code which is usually the same as the http status code
    #[serde(rename = "status")]
    pub status_code: u16,
}

impl std::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status_code, self.message)
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        ClientError::IoError(err.to_string())
    }
}
