use core::str::FromStr;

use binrw::{BinRead, BinWrite};

/// Modbus unit ID aka «slave ID».
#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq, BinRead, BinWrite)]
pub enum UnitId {
    /// Broadcast on a subnetwork. Also accepted for a direct connection.
    #[brw(magic(0_u8))]
    Broadcast,

    /// Direct connection.
    ///
    /// Note that some devices do not respond to it even with direct direction over local network.
    /// In that case, specify a [`Self::Significant`] unit ID explicitly.
    #[brw(magic(255_u8))]
    NonSignificant,

    /// Addressed unit ID. `248..=254` are reserved and not valid.
    #[bw(assert(matches!(self_0, 1..=247), "unit ID {self_0} is reserved"))]
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
