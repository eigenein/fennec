//! Sans-IO Modbus-over-TCP client decoders.

use binrw::{BinRead, io::Cursor};

use crate::{
    protocol,
    tcp,
    tcp::{Header, UnitId},
};

#[must_use]
pub struct TransportHeaderDecoder;

impl TransportHeaderDecoder {
    /// Receive the bytes from the wire.
    pub fn receive(self, bytes: &[u8; Header::SIZE]) -> Result<ResponsePayloadDecoder, tcp::Error> {
        let header = Header::read_be(&mut Cursor::new(bytes))?;
        Ok(ResponsePayloadDecoder(header))
    }
}

/// Awaiting the transaction payload state.
#[must_use]
pub struct ResponsePayloadDecoder(Header);

impl ResponsePayloadDecoder {
    /// Transaction ID of the upcoming response.
    #[must_use]
    pub const fn transaction_id(&self) -> u16 {
        self.0.transaction_id
    }

    /// Source unit ID of the upcoming response.
    pub const fn unit_id(&self) -> UnitId {
        self.0.unit_id
    }

    /// Expected response length.
    ///
    /// Transport implementors must read exactly this number of bytes and feed into [`Self::receive`].
    #[must_use]
    pub const fn n_expected_bytes(&self) -> u16 {
        self.0.length - 1
    }

    /// Receive the bytes from the wire and decode the response.
    pub fn receive<T: for<'a> BinRead<Args<'a> = ()>>(
        self,
        bytes: &[u8],
    ) -> (TransportHeaderDecoder, Result<Transaction<T>, tcp::Error>) {
        let n_expected_bytes = self.n_expected_bytes();
        let context = TransportHeaderDecoder;

        let result = if bytes.len() == usize::from(n_expected_bytes) {
            protocol::Response::<T>::read_be(&mut Cursor::new(bytes))
                .map(|response| Transaction { id: self.0.transaction_id, response })
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
        let context =
            TransportHeaderDecoder.receive(&[0x15, 0x01, 0x00, 0x00, 0x00, 0x09, 0xFF]).unwrap();
        assert_eq!(context.n_expected_bytes(), 8);
        assert_eq!(context.0.transaction_id, 0x1501);

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
