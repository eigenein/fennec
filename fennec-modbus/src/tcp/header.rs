use binrw::binrw;
use bon::Builder;

use crate::tcp::UnitId;

/// Modbus Application Protocol (Data Unit) header aka «MBAP header».
#[must_use]
#[binrw]
#[derive(Clone, Builder)]
#[brw(big)]
pub struct Header {
    pub transaction_id: u16,

    #[builder(default = 0)]
    pub protocol_id: u16,

    /// Number of following bytes, *including the Unit Identifier and data fields*.
    #[br(assert(length != 0))]
    #[bw(assert(*length != 0))]
    pub length: u16,

    /// Unit identifier aka «slave ID».
    ///
    /// Identification of a remote slave connected on a serial line or on other buses.
    pub unit_id: UnitId,
}

impl Header {
    pub const SIZE: usize = 7;
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use binrw::{BinRead, BinWrite, io::Cursor};

    use super::*;

    const ADU_BYTES: &[u8] = &[
        0x15, 0x01, // transaction ID: high, low
        0x00, 0x00, // protocol ID
        0x00, 0x06, // length
        0xFF, // unit ID
    ];

    #[test]
    fn read_example_ok() {
        let mut cursor = Cursor::new(ADU_BYTES);
        let adu = Header::read(&mut cursor).unwrap();
        assert_eq!(adu.transaction_id, 0x1501);
        assert_eq!(adu.protocol_id, 0);
        assert_eq!(adu.unit_id, UnitId::NonSignificant);
    }

    #[test]
    fn write_example_ok() {
        let mut cursor = Cursor::new(Vec::new());
        Header::builder()
            .unit_id(UnitId::NonSignificant)
            .transaction_id(0x1501)
            .length(6)
            .build()
            .write(&mut cursor)
            .unwrap();
        assert_eq!(cursor.into_inner(), ADU_BYTES);
    }
}
