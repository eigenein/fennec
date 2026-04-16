use alloc::vec::Vec;
use core::fmt::Debug;

use bytes::{Buf, BufMut};

use crate::{
    protocol,
    protocol::{Decode, Encode, function::ArgumentError},
};

#[must_use]
#[derive(Clone, Debug)]
pub struct Args {
    starting_address: u16,
    n_registers: u16,
    n_bytes: u8,
    words: Vec<u16>,
}

impl Args {
    pub fn new(starting_address: u16, words: Vec<u16>) -> Result<Self, ArgumentError> {
        let n_registers = words.len();
        if (1..=123).contains(&n_registers) {
            let n_registers = u16::try_from(n_registers)?;
            let n_bytes = u8::try_from(n_registers * 2)?;
            Ok(Self { starting_address, n_registers, n_bytes, words })
        } else {
            Err(ArgumentError::InvalidRegisterCount(n_registers))
        }
    }
}

impl Encode for Args {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u16(self.starting_address);
        buf.put_u16(self.n_registers);
        buf.put_u8(self.n_bytes);
        for word in &self.words {
            buf.put_u16(*word);
        }
    }
}

#[must_use]
#[derive(Copy, Clone, Debug)]
pub struct Output {
    pub starting_address: u16,
    pub n_registers: u16,
}

impl Decode for Output {
    type Output = Self;

    fn decode_from(buf: &mut impl Buf) -> Result<Self, protocol::Error> {
        Ok(Self { starting_address: buf.try_get_u16()?, n_registers: buf.try_get_u16()? })
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn request_example_ok() {
        const EXPECTED: &[u8] = &[
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
            0x04, // byte count
            0x00, 0x0A, // first word
            0x01, 0x02, // second word
        ];
        let bytes = Args::new(1, vec![0x000A, 0x0102]).unwrap().encode_into_bytes();
        assert_eq!(bytes, EXPECTED);
    }

    #[test]
    fn response_example_ok() {
        const RESPONSE: &[u8] = &[
            0x00, 0x01, // starting address: high, low
            0x00, 0x02, // register count: high, low
        ];

        #[expect(const_item_mutation)]
        let response = Output::decode_from(&mut RESPONSE).unwrap();

        assert_eq!(response.starting_address, 1);
        assert_eq!(response.n_registers, 2);
    }
}
