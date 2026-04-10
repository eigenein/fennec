//! Sans-IO Modbus-over-TCP client context.

use binrw::{BinRead, io::Cursor};

use crate::{protocol, tcp, tcp::Header};

/// Context that is awaiting an MBAP header.
#[must_use]
pub struct TransportHeaderExpectedContext;

impl TransportHeaderExpectedContext {
    /// Receive the bytes from the wire.
    pub fn receive(
        self,
        bytes: &[u8; Header::SIZE],
    ) -> Result<ProtocolResponseExpectedContext, tcp::Error> {
        let header = Header::read_be(&mut Cursor::new(bytes))?;
        Ok(ProtocolResponseExpectedContext { header })
    }
}

/// Context that is awaiting the transaction payload.
#[must_use]
pub struct ProtocolResponseExpectedContext {
    pub header: Header,
}

impl ProtocolResponseExpectedContext {
    /// Expected response length.
    #[must_use]
    pub const fn n_expected_bytes(&self) -> u16 {
        self.header.length - 1
    }

    /// Receive the bytes from the wire.
    pub fn receive<T: for<'a> BinRead<Args<'a> = ()>>(
        self,
        bytes: &[u8],
    ) -> (TransportHeaderExpectedContext, Result<Transaction<T>, tcp::Error>) {
        let n_expected_bytes = self.n_expected_bytes();
        let context = TransportHeaderExpectedContext;

        let result = if bytes.len() == usize::from(n_expected_bytes) {
            protocol::Response::<T>::read_be(&mut Cursor::new(bytes))
                .map(|response| Transaction { id: self.header.transaction_id, response })
                .map_err(protocol::Error::from)
                .map_err(tcp::Error::from)
        } else {
            Err(tcp::Error::PayloadSizeMismatch {
                n_expected_bytes: n_expected_bytes.into(),
                n_actual_bytes: bytes.len(),
            })
        };

        (context, result)
    }
}

#[must_use]
#[derive(Clone)]
pub struct Transaction<T: for<'a> BinRead<Args<'a> = ()>> {
    pub id: u16,
    pub response: protocol::Response<T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::function::read_holding_registers;

    #[test]
    fn receive_example_ok() {
        let context = TransportHeaderExpectedContext
            .receive(&[0x15, 0x01, 0x00, 0x00, 0x00, 0x09, 0xFF])
            .unwrap();
        assert_eq!(context.n_expected_bytes(), 8);
        assert_eq!(context.header.transaction_id, 0x1501);

        let (_, result) = context.receive::<read_holding_registers::Response>(&[
            0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64,
        ]);
        let transaction = result.unwrap();
        assert_eq!(transaction.id, 0x1501);

        let response = transaction.response.unwrap_ok();
        assert_eq!(response.n_bytes, 6);
        assert_eq!(response.words, [555, 0, 100]);
    }
}
