use alloc::vec::Vec;
use core::sync::atomic::{AtomicU16, Ordering};

use bytes::BufMut;

use crate::{
    Error,
    protocol::codec::Encode,
    tcp::{Header, UnitId},
};

/// Sans-IO Modbus-over-TCP transaction encoder used to prepare requests.
///
/// Under the hood, it uses simple incremental counter for transaction IDs.
#[must_use]
#[derive(Default)]
pub struct Encoder(AtomicU16);

impl Encoder {
    /// Instantiate the encoder starting with the specified transaction ID.
    pub const fn with_next_transaction_id(transaction_id: u16) -> Self {
        Self(AtomicU16::new(transaction_id))
    }

    /// Prepare the payload for sending.
    ///
    /// This wraps the payload, normally a [`crate::protocol::Request`],
    /// into an ADU and returns the respective transaction ID along.
    ///
    /// TCP transport implementors should send the resulting codec to the server.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bytes::BufMut;
    /// ///
    /// use fennec_modbus::{
    ///     protocol::codec,
    ///     tcp::{UnitId, transaction},
    /// };
    ///
    /// let encoder = transaction::Encoder::with_next_transaction_id(0x1501);
    /// let mut frame = Vec::new();
    /// let transaction_id = encoder.encode(UnitId::NonSignificant, &0x12345678, &mut frame).unwrap();
    ///
    /// assert_eq!(transaction_id, 0x1501);
    /// assert_eq!(
    ///     frame,
    ///     [
    ///         0x15, 0x01, // transaction ID: high, low
    ///         0x00, 0x00, // protocol ID
    ///         0x00, 0x05, // length: high, low
    ///         0xFF, // unit ID
    ///         0x12, 0x34, 0x56, 0x78, // request
    ///     ]
    /// );
    /// ```
    pub fn encode<P: Encode>(
        &self,
        unit_id: UnitId,
        payload: &P,
        to: &mut impl BufMut,
    ) -> Result<u16, Error> {
        let mut request_bytes = Vec::new();
        payload.encode(&mut request_bytes);

        let transaction_id = self.0.fetch_add(1, Ordering::Relaxed);
        let header = {
            let length = u16::try_from(request_bytes.len() + 1)
                .map_err(|_| Error::PayloadSizeExceeded(request_bytes.len()))?;
            Header { unit_id, transaction_id, length, protocol_id: Header::PROTOCOL_ID }
        };
        header.encode(to);
        to.put(&*request_bytes);

        Ok(transaction_id)
    }
}
