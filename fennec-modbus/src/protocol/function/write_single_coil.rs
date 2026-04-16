use bon::Builder;
use bytes::{Buf, BufMut};

use crate::protocol::{Decode, Encode, Error};

#[must_use]
#[derive(Builder, Copy, Clone, Debug)]
pub struct Payload {
    /// *Zero-based* address of the coil to write.
    address: u16,

    state: bool,
}

impl Encode for Payload {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u16(self.address);
        buf.put_u16(if self.state { 0xFF00 } else { 0x0000 });
    }
}

// TODO: wrap `bool`, discard `address`.
impl Decode for Payload {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self { address: buf.try_get_u16()?, state: buf.try_get_u16()? != 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0xAC, // address: high, low
        0xFF, 0x00, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let bytes = Payload::builder().address(172).state(true).build().encode_into_bytes();
        assert_eq!(bytes, PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        #[expect(const_item_mutation)]
        let response = Payload::decode_from(&mut PAYLOAD).unwrap();

        assert!(response.state);
    }
}
