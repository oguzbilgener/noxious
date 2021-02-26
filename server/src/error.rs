use serde::{Deserialize, Serialize};
use thiserror::Error;
use warp::http::StatusCode;

#[derive(Debug, Clone, Error, PartialEq, Serialize, Deserialize)]
pub enum StoreError {
    #[error("An item with this name already exists")]
    AlreadyExists,
    #[error("Item not found")]
    NotFound,
    #[error("Internal server error")]
    Other,
}

impl From<StoreError> for StatusCode {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::AlreadyExists => StatusCode::CONFLICT,
            StoreError::NotFound => StatusCode::NOT_FOUND,
            StoreError::Other => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
