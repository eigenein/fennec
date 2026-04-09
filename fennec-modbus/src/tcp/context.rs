//! Sans-IO Modbus-over-TCP client context.

use alloc::{collections::VecDeque, vec::Vec};

use binrw::{
    BinRead,
    BinWrite,
    io::{Cursor, Read, Seek, Write},
};

use crate::{Result, tcp::Header};

/// State-unaware context.
///
/// It is unsafe to use without tracking the connection state.
/// Hence, [`HeaderExpected`] and [`PduExpected`].
#[derive(Default)]
#[must_use]
pub struct Inner {
    transaction_counter: u16,

    /// Frames to get sent.
    ///
    /// Per the guidelines, we shouldn't try and send them concatenated. 😢
    ///
    /// TODO: expose `pop`.
    send_queue: VecDeque<Vec<u8>>,
}

impl Inner {
    /// Push the request to the send queue.
    ///
    /// Returns the transaction ID.
    pub fn send(&mut self, request: &impl for<'a> BinWrite<Args<'a> = ()>) -> Result<u16> {
        self.send_queue.push_back({
            let pdu_bytes = {
                let mut cursor = Cursor::new(Vec::new());
                request.write_be(&mut cursor)?;
                cursor.into_inner()
            };

            self.transaction_counter = self.transaction_counter.wrapping_add(1);

            let mut cursor = Cursor::new(Vec::new());
            Header::builder()
                .transaction_id(self.transaction_counter)
                .length(u16::try_from(pdu_bytes.len() + 1)?)
                .build()
                .write_be(&mut cursor)?;
            cursor.write_all(&pdu_bytes)?;
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
    pub fn on_header(self, header: &Header) -> PduExpected {
        PduExpected {
            inner: self.0,
            transaction_id: header.transaction_id,

            // FIXME: zero length is invalid.
            length: header.length.saturating_sub(1),
        }
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
    pub fn on_pdu<R: Read + Seek, P: for<'a> BinRead<Args<'a> = ()>>(
        self,
        reader: &mut R,
    ) -> Result<(HeaderExpected, u16, P)> {
        // FIXME: what if `read_be` fails?
        // FIXME: validate that exactly `length` is consumed.
        Ok((HeaderExpected(self.inner), self.transaction_id, P::read_be(reader)?))
    }
}
