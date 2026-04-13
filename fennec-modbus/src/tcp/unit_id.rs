use binrw::{BinRead, BinWrite};

use crate::tcp;

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

impl TryFrom<u8> for UnitId {
    type Error = tcp::Error;

    fn try_from(unit_id: u8) -> Result<Self, Self::Error> {
        match unit_id {
            0 => Ok(Self::Broadcast),
            255 => Ok(Self::NonSignificant),
            248..255 => Err(tcp::Error::InvalidUnitId(unit_id)),
            _ => Ok(Self::Significant(unit_id)),
        }
    }
}
