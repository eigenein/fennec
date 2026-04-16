use bon::Builder;
use bytes::{Buf, BufMut};

use crate::{
    protocol::{BitSize, Decode, Encode, Error},
    tcp::UnitId,
};

/// Modbus Application Protocol (Data Unit) header aka «MBAP header».
#[must_use]
#[derive(Clone, Builder)]
pub struct Header {
    /// Transaction ID used to match responses with requests.
    pub transaction_id: u16,

    /// Protocol ID. Always `0` for Modbus.
    #[builder(default = 0)]
    pub protocol_id: u16,

    /// Number of following bytes, *including the Unit Identifier and data fields*.
    pub length: u16,

    /// Unit identifier aka «slave ID».
    ///
    /// Identification of a remote slave connected on a serial line or on other buses.
    pub unit_id: UnitId,
}

impl Encode for Header {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u16(self.transaction_id);
        buf.put_u16(self.protocol_id);
        buf.put_u16(self.length);
        self.unit_id.encode_into(buf);
    }
}

impl Decode for Header {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self {
            transaction_id: buf.try_get_u16()?,
            protocol_id: buf.try_get_u16()?,
            length: buf.try_get_u16()?,
            unit_id: UnitId::decode_from(buf)?,
        })
    }
}

impl Header {
    /// Expected PDU length.
    ///
    /// TCP transport implementation should read exactly this number of bytes
    /// and parse as [`crate::protocol::data_unit::Response`].
    #[must_use]
    pub const fn payload_length(&self) -> u16 {
        self.length - 1
    }
}

impl BitSize for Header {
    const N_BITS: usize = Self::N_BYTES * 8;
    const N_BYTES: usize = 7;
}

#[cfg(test)]
mod tests {

    use super::*;

    const BYTES: &[u8] = &[
        0x15, 0x01, // transaction ID: high, low
        0x00, 0x00, // protocol ID
        0x00, 0x06, // length
        0xFF, // unit ID
    ];

    #[test]
    fn read_example_ok() {
        #[expect(const_item_mutation)]
        let header = Header::decode_from(&mut BYTES).unwrap();
        assert_eq!(header.transaction_id, 0x1501);
        assert_eq!(header.protocol_id, 0);
        assert_eq!(header.unit_id, UnitId::NonSignificant);
    }

    #[test]
    fn write_example_ok() {
        let bytes = Header::builder()
            .unit_id(UnitId::NonSignificant)
            .transaction_id(0x1501)
            .length(6)
            .build()
            .encode_into_bytes();
        assert_eq!(bytes, BYTES);
    }
}
