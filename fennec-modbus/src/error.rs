use core::num::TryFromIntError;

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

    #[error("could not convert the quantity: {0}")]
    QuantityConversion(#[from] TryFromIntError),
}
