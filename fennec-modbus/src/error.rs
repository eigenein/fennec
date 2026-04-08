use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to build the request: {0}")]
    RequestBuilder(#[from] RequestBuilderError),
}

/// Request construction failed before sending it.
#[derive(Debug, Error)]
pub enum RequestBuilderError {
    #[error("incorrect number of registers requested ({0})")]
    RegisterCount(u16),
}
