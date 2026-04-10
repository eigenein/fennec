//! Sans-IO Modbus-over-TCP client context.

use alloc::{collections::VecDeque, vec::Vec};

use binrw::{
    BinRead,
    BinWrite,
    io::{Cursor, Write},
};

use crate::{
    protocol,
    tcp,
    tcp::{Header, UnitId},
};

/// State-unaware context.
///
/// It is unsafe to use without tracking the connection state.
/// Hence, [`TransportHeaderExpected`] and [`ProtocolResponseExpected`].
#[derive(Default)]
#[must_use]
pub struct Inner {
    next_transaction_id: u16,

    /// Frames queued for sending over the wire.
    ///
    /// Per the guidelines, we shouldn't try and send them concatenated. 😢
    send_queue: VecDeque<Vec<u8>>,
}

impl Inner {
    pub const fn with_next_transaction_id(next_transaction_id: u16) -> Self {
        Self { next_transaction_id, send_queue: VecDeque::new() }
    }

    /// Pop a frame for sending over the wire, if any.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.send_queue.pop_front()
    }

    /// Push the request to the send queue.
    ///
    /// This wraps the payload into an ADU and returns the transaction ID.
    pub fn send(
        &mut self,
        unit_id: UnitId,
        request: &impl for<'a> BinWrite<Args<'a> = ()>,
    ) -> Result<u16, tcp::Error> {
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
                .transaction_id(self.next_transaction_id)
                .length(length)
                .build()
        };

        self.send_queue.push_back({
            let mut frame_cursor = Cursor::new(Vec::new());
            header.write_be(&mut frame_cursor)?;
            frame_cursor.write_all(&payload_bytes).map_err(binrw::Error::Io)?;
            frame_cursor.into_inner()
        });

        self.next_transaction_id = self.next_transaction_id.wrapping_add(1);
        Ok(header.transaction_id)
    }
}

/// Context that is awaiting an MBAP header.
#[derive(Default, derive_more::Deref)]
#[must_use]
pub struct TransportHeaderExpected(Inner);

impl TransportHeaderExpected {
    /// Receive the bytes from the wire.
    pub fn receive(
        self,
        bytes: &[u8; Header::SIZE],
    ) -> Result<ProtocolResponseExpected, tcp::Error> {
        let header = Header::read_be(&mut Cursor::new(bytes))?;
        Ok(ProtocolResponseExpected {
            inner: self.0,
            transaction_id: header.transaction_id,
            length: header.length - 1,
        })
    }
}

/// Context that is awaiting the transaction payload.
#[must_use]
#[derive(derive_more::Deref)]
pub struct ProtocolResponseExpected {
    #[deref]
    inner: Inner,

    transaction_id: u16,

    /// PDU length.
    pub length: u16,
}

impl ProtocolResponseExpected {
    /// Receive the bytes from the wire.
    pub fn receive<P: for<'a> BinRead<Args<'a> = ()>>(
        self,
        bytes: &[u8],
    ) -> (TransportHeaderExpected, Result<Transaction<P>, tcp::Error>) {
        let context = TransportHeaderExpected(self.inner);

        let result = if bytes.len() == usize::from(self.length) {
            P::read_be(&mut Cursor::new(bytes))
                .map(|payload| Transaction { id: self.transaction_id, payload })
                .map_err(protocol::Error::from)
                .map_err(tcp::Error::from)
        } else {
            Err(tcp::Error::PayloadSizeMismatch {
                n_expected_bytes: self.length.into(),
                n_actual_bytes: bytes.len(),
            })
        };

        (context, result)
    }
}

#[derive(Clone)]
pub struct Transaction<P> {
    pub id: u16,
    pub payload: P,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::function::read_holding_registers;

    #[test]
    fn send_example_ok() {
        let mut context = Inner::with_next_transaction_id(0x1501);
        let request = read_holding_registers::Request::builder()
            .starting_address(4)
            .n_registers(1)
            .build()
            .unwrap();
        let transaction_id = context.send(UnitId::NonSignificant, &request).unwrap();

        assert_eq!(transaction_id, 0x1501);
        assert_eq!(context.next_transaction_id, 0x1502);

        let frame = context.pop().unwrap();
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
