#![allow(dead_code)]

use binrw::{BinRead, BinWrite};
use bon::Builder;

use crate::pdu;

/// Write a single output to either «on» or «off» in a remote device.
#[derive(Copy, Clone)]
pub struct Function;

impl pdu::Function for Function {
    const CODE: u8 = 5;
    type Request = Request;
    type Response = Response;
}

pub type Request = Payload;
pub type Response = Payload;

#[must_use]
#[derive(Builder, Copy, Clone, Debug, BinRead, BinWrite)]
#[brw(big, magic = 5_u8)]
pub struct Payload {
    /// *Zero-based* address of the coil to write.
    address: u16,

    #[br(map = |it: u16| it == 0xFF00)]
    #[bw(map = |it: &bool| if *it { 0xFF00u16 } else { 0x0000u16 })]
    state: bool,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    const PAYLOAD: &[u8] = &[
        0x05, // function code
        0x00, 0xAC, // address: high, low
        0xFF, 0x00, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let mut output = Cursor::new(vec![]);
        Request::builder().address(172).state(true).build().write(&mut output).unwrap();
        assert_eq!(output.into_inner(), PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        let response = Response::read(&mut Cursor::new(PAYLOAD)).unwrap();
        assert!(response.state);
    }
}
