#![allow(dead_code)]

use core::fmt::Debug;

use binrw::{BinWrite, binread};

use crate::pdu;

/// Read the contents of eight Exception Status outputs in a remote device.
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 7;
    type Request = Request;
    type Response = Response;
}

#[must_use]
#[derive(Copy, Clone, Debug, BinWrite)]
#[bw(big, magic = 7_u8)]
pub struct Request;

#[must_use]
#[binread]
#[br(big, magic = 7_u8)]
#[derive(derive_more::Debug)]
pub struct Response {
    /// Status of the eight Exception Status outputs.
    ///
    /// The contents of the eight Exception Status outputs are device specific.
    pub output: u8,
}

impl From<Response> for u8 {
    fn from(response: Response) -> Self {
        response.output
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x07, // function code
        ];
        let mut output = Cursor::new(vec![]);
        Request.write(&mut output).unwrap();
        assert_eq!(output.into_inner(), EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x07, // function code
            0x6D, // output
        ];
        let response = Response::read(&mut Cursor::new(RESPONSE)).unwrap();
        assert_eq!(response.output, 0x6D);
    }
}
