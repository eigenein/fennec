use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad request: {0}")]
    BadRequest(#[from] BadRequest),
}

#[derive(Debug, Error)]
pub enum BadRequest {
    #[error("incorrect number of registers requested ({0})")]
    RegisterCount(u16),
}
