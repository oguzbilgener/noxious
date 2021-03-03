use thiserror::Error;

/// Generic not found error
#[derive(Error, Debug, Clone, Copy)]
#[error("Item not found")]
pub struct NotFoundError;

/// Toxic update failed
#[derive(Debug, Clone, Copy, Error, PartialEq)]
pub enum ToxicUpdateError {
    /// No such toxic with the given name
    #[error("Toxic not found")]
    NotFound,
    /// Some other error
    #[error("Other error")]
    Other,
}

impl From<NotFoundError> for ToxicUpdateError {
    fn from(_: NotFoundError) -> Self {
        ToxicUpdateError::NotFound
    }
}
