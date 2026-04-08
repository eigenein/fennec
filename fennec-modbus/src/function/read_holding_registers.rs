//! Section 6.3 «Read Holding Registers».

use alloc::vec::Vec;

use binrw::{BinWrite, binread};

#[must_use]
#[derive(Copy, Clone, BinWrite)]
#[bw(big, magic = 3_u8)]
pub struct Request {
    #[bw(map = |it: &u16| it - 1_u16)]
    pub starting_address: u16,

    pub count: u16,
}

#[must_use]
#[binread]
#[br(big, magic = 3_u8)]
pub struct Response {
    #[br(temp)]
    byte_count: u8,

    #[br(count = byte_count / 2)]
    pub words: Vec<u16>,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn write_example_ok() {
        const REQUEST: Request = Request { starting_address: 108, count: 3 };
        const EXPECTED: &[u8] = &[
            0x03, // function code
            0x00, 0x6B, // starting address: high, low
            0x00, 0x03, // count: high, low
        ];
        let mut output = Cursor::new(vec![]);
        REQUEST.write(&mut output).unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn read_example_ok() {
        const RESPONSE: &[u8] = &[
            0x03, // function code
            0x06, // byte count
            0x02, 0x2B, // value: high, low
            0x00, 0x00, // value: high, low
            0x00, 0x64, // value: high, low
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.words, [555, 0, 100]);
    }
}
