use bon::Builder;
use deku::{DekuRead, DekuWrite};

#[must_use]
#[derive(Builder, Copy, Clone, Debug, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Payload {
    /// *Zero-based* address of the register to write.
    address: u16,

    value: u16,
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0x01, // address: high, low
        0x00, 0x03, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let bytes = Payload::builder().address(1).value(3).build().to_bytes().unwrap();
        assert_eq!(bytes, PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        let (_, payload) = Payload::from_bytes((PAYLOAD, 0)).unwrap();
        assert_eq!(payload.address, 1);
        assert_eq!(payload.value, 3);
    }
}
