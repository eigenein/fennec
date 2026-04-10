//! Sans-IO Modbus-over-TCP client.

mod codec;
pub mod context;
mod error;
mod header;
mod unit_id;

#[cfg(feature = "tokio")]
mod tokio;

pub use self::{codec::Codec, error::Error, header::Header, unit_id::UnitId};
