use core::str::FromStr;

use deku::{DekuRead, DekuSize, DekuWrite};

/// Modbus unit ID aka «slave ID».
#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuSize, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "big")]
pub enum UnitId {
    /// Broadcast on a subnetwork. Also accepted for a direct connection.
    #[deku(id = 0)]
    Broadcast,

    /// Direct connection.
    ///
    /// Note that some devices do not respond to it even with direct direction over local network.
    /// In that case, specify a [`Self::Significant`] unit ID explicitly.
    #[deku(id = 255)]
    NonSignificant,

    /// Addressed unit ID. `248..=254` are reserved and not valid.
    #[deku(id_pat = "1..=247")]
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
