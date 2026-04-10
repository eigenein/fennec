//! Sans-IO Modbus-over-TCP client.

mod decoder;
mod encoder;
mod error;
mod header;
mod unit_id;

#[cfg(feature = "tokio")]
mod tokio;

pub use self::{decoder::*, encoder::Encoder, error::Error, header::Header, unit_id::UnitId};
