//! Sans-IO Modbus-over-TCP client context.

use alloc::{collections::VecDeque, vec::Vec};

use binrw::{
    BinRead,
    BinWrite,
    io::{Cursor, Write},
};

use crate::{Error, Result, tcp::Header};

/// State-unaware context.
///
/// It is unsafe to use without tracking the connection state.
/// Hence, [`HeaderExpected`] and [`PduExpected`].
#[derive(Default)]
#[must_use]
pub struct Inner {
    transaction_counter: u16,

    /// ADU's queued for sending over the wire.
    ///
    /// Per the guidelines, we shouldn't try and send them concatenated. 😢
    send_queue: VecDeque<Vec<u8>>,
}

impl Inner {
    /// Pop a byte chunk for sending over the wire, if any.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.send_queue.pop_front()
    }

    /// Push the request to the send queue.
    ///
    /// This wraps the payload into an ADU and returns the transaction ID.
    pub fn send(&mut self, request: &impl for<'a> BinWrite<Args<'a> = ()>) -> Result<u16> {
        self.transaction_counter = self.transaction_counter.wrapping_add(1);

        self.send_queue.push_back({
            let request_bytes = {
                let mut cursor = Cursor::new(Vec::new());
                request.write_be(&mut cursor)?;
                cursor.into_inner()
            };

            let mut cursor = Cursor::new(Vec::new());
            let length = request_bytes.len() + 1;
            Header::builder()
                .transaction_id(self.transaction_counter)
                .length(u16::try_from(length).map_err(|_| Error::InvalidLength(length))?)
                .build()
                .write_be(&mut cursor)?;
            cursor.write_all(&request_bytes)?;
            cursor.into_inner()
        });

        Ok(self.transaction_counter)
    }
}

/// Context that is awaiting an MBAP header.
#[derive(Default, derive_more::Deref)]
#[must_use]
pub struct HeaderExpected(Inner);

impl HeaderExpected {
    /// Receive the bytes from the wire.
    pub fn receive(self, bytes: &[u8; Header::SIZE]) -> Result<PduExpected> {
        let header = Header::read_be(&mut Cursor::new(bytes))?;
        Ok(PduExpected {
            inner: self.0,
            transaction_id: header.transaction_id,
            length: header.length - 1,
        })
    }
}

/// Context that is awaiting the transaction payload.
#[must_use]
#[derive(derive_more::Deref)]
pub struct PduExpected {
    #[deref]
    inner: Inner,

    transaction_id: u16,

    /// PDU length.
    pub length: u16,
}

impl PduExpected {
    /// Receive the bytes from the wire.
    pub fn receive<P: for<'a> BinRead<Args<'a> = ()>>(
        self,
        bytes: &[u8],
    ) -> Result<(HeaderExpected, u16, P)> {
        if bytes.len() != usize::from(self.length) {
            return Err(Error::PayloadSizeMismatch {
                n_expected_bytes: self.length.into(),
                n_actual_bytes: bytes.len(),
            });
        }
        Ok((HeaderExpected(self.inner), self.transaction_id, P::read_be(&mut Cursor::new(bytes))?))
    }
}
