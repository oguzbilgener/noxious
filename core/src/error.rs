use thiserror::Error;

#[derive(Error, Debug)]
#[error("Item not found")]
pub struct NotFoundError;
