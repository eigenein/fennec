use bytes::{Buf, BufMut};

use crate::protocol::{Decode, Encode, Error};

#[must_use]
#[derive(Copy, Clone, Debug)]
pub struct Payload {
    /// *Zero-based* address of the register to write.
    pub address: u16,

    pub value: u16,
}

impl Encode for Payload {
    fn encode_into(&self, buf: &mut impl BufMut) {
        buf.put_u16(self.address);
        buf.put_u16(self.value);
    }
}

impl Decode for Payload {
    type Output = Self;

    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self { address: buf.try_get_u16()?, value: buf.try_get_u16()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0x01, // address: high, low
        0x00, 0x03, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let payload = Payload { address: 1, value: 3 };
        let bytes = payload.encode_into_bytes();
        assert_eq!(bytes, PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        #[expect(const_item_mutation)]
        let response = Payload::decode_from(&mut PAYLOAD).unwrap();

        assert_eq!(response.address, 1);
        assert_eq!(response.value, 3);
    }
}
