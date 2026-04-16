use alloc::vec::Vec;
use core::sync::atomic::{AtomicU16, Ordering};

use bytes::BufMut;

use crate::{
    protocol::Encode,
    tcp,
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
    /// This wraps the payload, normally a [`crate::protocol::data_unit::Request`],
    /// into an ADU and returns the respective transaction ID along.
    ///
    /// TCP transport implementors should send the resulting bytes to the server.
    pub fn wrap(
        &self,
        unit_id: UnitId,
        request: &impl Encode,
    ) -> Result<(Vec<u8>, u16), tcp::Error> {
        let transaction_id = self.0.fetch_add(1, Ordering::Relaxed);
        let frame_bytes = {
            let payload_bytes = request.encode_into_bytes();
            let mut frame_bytes = {
                let length = u16::try_from(payload_bytes.len() + 1)
                    .map_err(|_| tcp::Error::PayloadSizeExceeded(payload_bytes.len()))?;
                Header::builder()
                    .unit_id(unit_id)
                    .transaction_id(transaction_id)
                    .length(length)
                    .build()
                    .encode_into_bytes()
            };
            frame_bytes.put(&*payload_bytes);
            frame_bytes
        };
        Ok((frame_bytes, transaction_id))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn send_example_ok() {
        let encoder = Encoder::with_next_transaction_id(0x1501);
        let (frame, transaction_id) =
            encoder.wrap(UnitId::NonSignificant, &[0x03, 0x00, 0x04, 0x00, 0x01]).unwrap();

        assert_eq!(transaction_id, 0x1501);
        assert_eq!(encoder.0.into_inner(), 0x1502);
        assert_eq!(
            frame,
            [
                0x15, 0x01, // transaction ID: high, low
                0x00, 0x00, // protocol ID
                0x00, 0x06, // length: high, low
                0xFF, // unit ID
                0x03, 0x00, 0x04, 0x00, 0x01, // request
            ]
        );
    }
}
