use bon::Builder;
use deku::{DekuRead, DekuSize, DekuWrite};

use crate::tcp::UnitId;

/// Modbus Application Protocol (Data Unit) header aka «MBAP header».
#[must_use]
#[derive(Clone, Builder, DekuRead, DekuWrite, DekuSize)]
#[deku(endian = "big")]
pub struct Header {
    /// Transaction ID used to match responses with requests.
    pub transaction_id: u16,

    /// Protocol ID. Always `0` for Modbus.
    #[builder(default = 0)]
    #[deku(assert_eq = "0")]
    pub protocol_id: u16,

    /// Number of following bytes, *including the Unit Identifier and data fields*.
    #[deku(assert = "*length != 0")]
    pub length: u16,

    /// Unit identifier aka «slave ID».
    ///
    /// Identification of a remote slave connected on a serial line or on other buses.
    pub unit_id: UnitId,
}

impl Header {
    pub const N_BYTES: usize = Self::SIZE_BYTES.unwrap();

    /// Expected PDU length.
    ///
    /// TCP transport implementation should read exactly this number of bytes
    /// and parse as [`crate::protocol::data_unit::Response`].
    #[must_use]
    pub const fn payload_length(&self) -> u16 {
        self.length - 1
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    const ADU_BYTES: &[u8] = &[
        0x15, 0x01, // transaction ID: high, low
        0x00, 0x00, // protocol ID
        0x00, 0x06, // length
        0xFF, // unit ID
    ];

    #[test]
    fn read_example_ok() {
        let (_, adu) = Header::from_bytes((ADU_BYTES, 0)).unwrap();
        assert_eq!(adu.transaction_id, 0x1501);
        assert_eq!(adu.protocol_id, 0);
        assert_eq!(adu.unit_id, UnitId::NonSignificant);
    }

    #[test]
    fn write_example_ok() {
        let bytes = Header::builder()
            .unit_id(UnitId::NonSignificant)
            .transaction_id(0x1501)
            .length(6)
            .build()
            .to_bytes()
            .unwrap();
        assert_eq!(bytes, ADU_BYTES);
    }
}
