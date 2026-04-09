use alloc::string::{String, ToString};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to build the request: {0}")]
    RequestBuilder(#[from] RequestBuilderError),
}

/// Request construction failed before sending it.
#[derive(Debug, Error)]
pub enum RequestBuilderError {
    #[error("incorrect quantity requested ({0})")]
    InvalidQuantity(u16),

    #[error("failed to serialize the payload: {0}")]
    Serialization(String),
}

impl From<binrw::Error> for RequestBuilderError {
    fn from(err: binrw::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
