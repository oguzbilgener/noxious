use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Item not found")]
pub struct NotFoundError;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum ToxicUpdateError {
    #[error("Toxic not found")]
    NotFound,
    #[error("Other error")]
    Other,
}

impl From<NotFoundError> for ToxicUpdateError {
    fn from(_: NotFoundError) -> Self {
        ToxicUpdateError::NotFound
    }
}
