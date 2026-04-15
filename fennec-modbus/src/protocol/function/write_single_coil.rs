use bon::Builder;
use deku::{DekuRead, DekuWrite};

#[must_use]
#[derive(Builder, Copy, Clone, Debug, DekuRead, DekuWrite)]
pub struct Payload {
    /// *Zero-based* address of the coil to write.
    #[deku(endian = "big")]
    address: u16,

    state: State,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "big", id_type = "u16")]
pub enum State {
    #[deku(id = 0x0000)]
    Off,

    #[deku(id = 0xFF00)]
    On,
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    const PAYLOAD: &[u8] = &[
        0x00, 0xAC, // address: high, low
        0xFF, 0x00, // output value: high, low
    ];

    #[test]
    fn request_example_ok() {
        let bytes = Payload::builder().address(172).state(State::On).build().to_bytes().unwrap();
        assert_eq!(bytes, PAYLOAD);
    }

    #[test]
    fn response_example_ok() {
        let (_, response) = Payload::from_bytes((PAYLOAD, 0)).unwrap();
        assert_eq!(response.state, State::On);
    }
}
