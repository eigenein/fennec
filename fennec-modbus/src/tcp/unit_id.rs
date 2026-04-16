use core::str::FromStr;

use bytes::{Buf, BufMut};

use crate::protocol::{Decode, Encode, Error};

/// Modbus unit ID aka «slave ID».
#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UnitId {
    /// Broadcast on a subnetwork. Also accepted for a direct connection.
    Broadcast,

    /// Direct connection.
    ///
    /// Note that some devices do not respond to it even with direct direction over local network.
    /// In that case, specify a [`Self::Significant`] unit ID explicitly.
    NonSignificant,

    /// Addressed unit ID. `248..=254` are reserved and not valid.
    Significant(u8),
}

impl From<u8> for UnitId {
    fn from(unit_id: u8) -> Self {
        match unit_id {
            0 => Self::Broadcast,
            255 => Self::NonSignificant,
            _ => Self::Significant(unit_id),
        }
    }
}

impl FromStr for UnitId {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(u8::from_str(s)?))
    }
}

impl Encode for UnitId {
    fn encode_into(&self, buf: &mut impl BufMut) {
        match self {
            Self::Broadcast => buf.put_u8(0),
            Self::NonSignificant => buf.put_u8(255),
            Self::Significant(unit_id) => buf.put_u8(*unit_id),
        }
    }
}

impl Decode for UnitId {
    type Output = Self;

    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(buf.try_get_u8()?.into())
    }
}
