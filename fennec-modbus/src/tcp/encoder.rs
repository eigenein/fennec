use alloc::vec::Vec;
use core::sync::atomic::{AtomicU16, Ordering};

use binrw::{
    BinWrite,
    io::{Cursor, Write},
};

use crate::{
    tcp,
    tcp::{Header, UnitId},
};

/// Sans-IO Modbus-over-TCP transaction encoder used to prepare requests.
#[must_use]
#[derive(Default)]
pub struct Encoder(AtomicU16);

impl Encoder {
    pub const fn with_next_transaction_id(transaction_id: u16) -> Self {
        Self(AtomicU16::new(transaction_id))
    }

    /// Prepare the request.
    ///
    /// This wraps the payload into an ADU and returns the respective transaction ID.
    pub fn prepare(
        &mut self,
        unit_id: UnitId,
        request: &impl for<'a> BinWrite<Args<'a> = ()>,
    ) -> Result<(Vec<u8>, u16), tcp::Error> {
        let payload_bytes = {
            let mut cursor = Cursor::new(Vec::new());
            request.write_be(&mut cursor)?;
            cursor.into_inner()
        };

        let header = {
            let length = u16::try_from(payload_bytes.len() + 1)
                .map_err(|_| tcp::Error::PayloadSizeExceeded(payload_bytes.len()))?;
            Header::builder()
                .unit_id(unit_id)
                .transaction_id(self.0.fetch_add(1, Ordering::Relaxed))
                .length(length)
                .build()
        };

        let frame_bytes = {
            let mut frame_cursor = Cursor::new(Vec::new());
            header.write_be(&mut frame_cursor)?;
            frame_cursor.write_all(&payload_bytes).map_err(binrw::Error::Io)?;
            frame_cursor.into_inner()
        };

        Ok((frame_bytes, header.transaction_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::function::read_holding_registers;

    #[test]
    fn send_example_ok() {
        let mut codec = Encoder::with_next_transaction_id(0x1501);
        let request = read_holding_registers::Request::builder()
            .starting_address(4)
            .n_registers(1)
            .build()
            .unwrap();
        let (frame, transaction_id) = codec.prepare(UnitId::NonSignificant, &request).unwrap();

        assert_eq!(transaction_id, 0x1501);
        assert_eq!(codec.0.into_inner(), 0x1502);
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
