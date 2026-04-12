use alloc::{boxed::Box, format};

use binrw::{BinRead, BinWrite};
use bon::Builder;

use crate::protocol;

/// Write a single output to either «on» or «off» in a remote device.
#[must_use]
pub struct Function;

impl protocol::Function for Function {
    const CODE: u8 = 5;
    type Args = Payload;
    type Output = Payload;
}

#[must_use]
#[derive(Builder, Copy, Clone, Debug, BinRead, BinWrite)]
#[brw(big)]
pub struct Payload {
    /// *Zero-based* address of the coil to write.
    address: u16,

    #[br(try_map = |it: u16| match it {
        0xFF00 => Ok(true),
        0x0000 => Ok(false),
        other => Err(format!("invalid coil value: 0x{other:04X}")),
    })]
    #[bw(map = |it: &bool| if *it { 0xFF00u16 } else { 0x0000u16 })]
    state: bool,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use binrw::{BinRead, io::Cursor};

    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0xAC, // address: high, low
        0xFF, 0x00, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let mut output = Cursor::new(vec![]);
        Payload::builder().address(172).state(true).build().write(&mut output).unwrap();
        assert_eq!(output.into_inner(), PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        let response = Payload::read(&mut Cursor::new(PAYLOAD)).unwrap();
        assert!(response.state);
    }
}
