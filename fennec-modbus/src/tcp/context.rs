//! Sans-IO Modbus-over-TCP client context.

use alloc::{collections::VecDeque, vec::Vec};

use binrw::{BinRead, BinWrite, io::Cursor};

use crate::{protocol, tcp, tcp::Header};

/// State-unaware context.
///
/// It is unsafe to use without tracking the connection state.
/// Hence, [`HeaderExpected`] and [`PayloadExpected`].
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
    pub fn send(
        &mut self,
        request: &impl for<'a> BinWrite<Args<'a> = ()>,
    ) -> Result<u16, tcp::Error> {
        self.transaction_counter = self.transaction_counter.wrapping_add(1);
        self.send_queue.push_back({
            let payload_bytes = {
                let mut cursor = Cursor::new(Vec::new());
                request.write_be(&mut cursor)?;
                cursor.into_inner()
            };
            let length = u16::try_from(payload_bytes.len() + 1)
                .map_err(|_| tcp::Error::PayloadSizeExceeded(payload_bytes.len()))?;
            let mut cursor = Cursor::new(payload_bytes);
            Header::builder()
                .transaction_id(self.transaction_counter)
                .length(length)
                .build()
                .write_be(&mut cursor)?;
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
    pub fn receive(self, bytes: &[u8; Header::SIZE]) -> Result<PayloadExpected, tcp::Error> {
        let header = Header::read_be(&mut Cursor::new(bytes))?;
        Ok(PayloadExpected {
            inner: self.0,
            transaction_id: header.transaction_id,
            length: header.length - 1,
        })
    }
}

/// Context that is awaiting the transaction payload.
#[must_use]
#[derive(derive_more::Deref)]
pub struct PayloadExpected {
    #[deref]
    inner: Inner,

    transaction_id: u16,

    /// PDU length.
    pub length: u16,
}

impl PayloadExpected {
    /// Receive the bytes from the wire.
    pub fn receive<P: for<'a> BinRead<Args<'a> = ()>>(
        self,
        bytes: &[u8],
    ) -> (HeaderExpected, Result<Transaction<P>, tcp::Error>) {
        let context = HeaderExpected(self.inner);

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
