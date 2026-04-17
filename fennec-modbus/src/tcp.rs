//! Sans-IO Modbus-over-TCP client.

pub mod header;
pub mod tokio;
pub mod transaction;
mod unit_id;

pub use self::unit_id::UnitId;

/// Modbus Application Protocol (Data Unit) header aka «MBAP header».
#[must_use]
#[derive(Clone)]
pub struct Header {
    /// Transaction ID used to match responses with requests.
    pub transaction_id: u16,

    /// Protocol ID. Always `0` for Modbus.
    pub protocol_id: u16,

    /// Number of following codec, *including the Unit Identifier and data fields*.
    pub length: u16,

    /// Unit identifier aka «slave ID».
    ///
    /// Identification of a remote slave connected on a serial line or on other buses.
    pub unit_id: UnitId,
}

impl Header {
    pub const PROTOCOL_ID: u16 = 0;
}
