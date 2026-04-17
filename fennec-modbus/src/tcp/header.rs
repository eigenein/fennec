use bytes::{Buf, BufMut};

use crate::{Error, protocol::codec, tcp::Header};

pub struct Encoder;

impl codec::Encoder<Header> for Encoder {
    /// Encode the header.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::{
    ///     protocol::codec::Decoder,
    ///     tcp::{UnitId, header},
    /// };
    ///
    /// let mut bytes: &[u8] = &[
    ///     0x15, 0x01, // transaction ID: high, low
    ///     0x00, 0x00, // protocol ID
    ///     0x00, 0x06, // length
    ///     0xFF, // unit ID
    /// ];
    /// let header = header::Decoder::decode(&mut bytes).unwrap();
    ///
    /// assert_eq!(header.transaction_id, 0x1501);
    /// assert_eq!(header.protocol_id, 0);
    /// assert_eq!(header.unit_id, UnitId::NonSignificant);
    /// ```
    fn encode(header: &Header, buf: &mut impl BufMut) {
        buf.put_u16(header.transaction_id);
        buf.put_u16(header.protocol_id);
        buf.put_u16(header.length);
        buf.put_u8(header.unit_id.into());
    }
}

pub struct Decoder;

impl codec::Decoder<Header> for Decoder {
    /// Decode a header.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fennec_modbus::{
    ///     protocol::codec::Encoder,
    ///     tcp::{Header, UnitId, header},
    /// };
    ///
    /// const EXPECTED: &[u8] = &[
    ///     0x15, 0x01, // transaction ID: high, low
    ///     0x00, 0x00, // protocol ID
    ///     0x00, 0x06, // length
    ///     0xFF, // unit ID
    /// ];
    ///
    /// let header = Header {
    ///     unit_id: UnitId::NonSignificant,
    ///     transaction_id: 0x1501,
    ///     length: 6,
    ///     protocol_id: 0,
    /// };
    /// let mut bytes = Vec::new();
    /// header::Encoder::encode(&header, &mut bytes);
    /// assert_eq!(bytes, EXPECTED);
    /// ```
    fn decode(from: &mut impl Buf) -> Result<Header, Error> {
        Ok(Header {
            transaction_id: from.try_get_u16()?,
            protocol_id: from.try_get_u16()?,
            length: from.try_get_u16()?,
            unit_id: from.try_get_u8()?.into(),
        })
    }
}

impl Header {
    pub const N_BYTES: usize = 7;

    /// Expected PDU length.
    ///
    /// TCP transport implementation should read exactly this number of codec
    /// and parse as [`crate::protocol::Response`].
    #[must_use]
    pub const fn payload_length(&self) -> u16 {
        self.length - 1
    }
}
