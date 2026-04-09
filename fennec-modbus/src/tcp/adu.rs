use alloc::{boxed::Box, vec::Vec};

use binrw::binrw;
use bon::Builder;

use crate::tcp::UnitId;

/// Modbus Application Data Unit.
#[must_use]
#[binrw]
#[derive(Clone, Builder)]
#[brw(big)]
pub struct Adu {
    pub transaction_id: u16,

    #[builder(default = 0)]
    pub protocol_id: u16,

    /// Number of following bytes, *including the Unit Identifier and data fields*.
    #[bw(try_calc = u16::try_from(payload.len() + 1))]
    #[builder(skip)]
    pub length: u16,

    /// Unit identifier aka «slave ID».
    ///
    /// Identification of a remote slave connected on a serial line or on other buses.
    #[builder(default = UnitId::NonSignificant)]
    pub unit_id: UnitId,

    #[br(count = length.saturating_sub(1))]
    pub payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, BinWrite, io::Cursor};

    use super::*;

    const ADU_BYTES: &[u8] = &[
        0x15, 0x01, // transaction ID: high, low
        0x00, 0x00, // protocol ID
        0x00, 0x06, // length
        0xFF, // unit ID
        0x03, // function code
        0x00, 0x04, // starting address
        0x00, 0x01, // number of registers
    ];

    #[test]
    fn read_example_ok() {
        let mut cursor = Cursor::new(ADU_BYTES);
        let adu = Adu::read(&mut cursor).unwrap();
        assert_eq!(adu.transaction_id, 0x1501);
        assert_eq!(adu.protocol_id, 0);
        assert_eq!(adu.unit_id, UnitId::NonSignificant);
        assert_eq!(adu.payload, vec![0x03, 0x00, 0x04, 0x00, 0x01]);
    }

    #[test]
    fn write_example_ok() {
        let mut cursor = Cursor::new(Vec::new());
        Adu::builder()
            .transaction_id(0x1501)
            .payload(vec![0x03, 0x00, 0x04, 0x00, 0x01])
            .build()
            .write(&mut cursor)
            .unwrap();
        assert_eq!(cursor.into_inner(), ADU_BYTES);
    }
}
