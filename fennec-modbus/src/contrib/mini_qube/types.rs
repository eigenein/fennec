//! MiniQube-specific types.

use bytes::Buf;

use crate::{
    Error,
    contrib::types::Percentage,
    protocol::codec::{BitSize, Decode},
};

#[must_use]
#[derive(Copy, Clone)]
pub struct StateOfChargeSettings {
    /// Minimum system state-of-charge.
    pub min_system: Percentage<u16>,

    /// Minimum state-of-charge on grid.
    pub min_on_grid: Percentage<u16>,

    /// Maximum state-of-charge.
    pub max: Percentage<u16>,
}

impl BitSize for StateOfChargeSettings {
    const N_BITS: u16 = u16::N_BITS * Self::N_WORDS;
    const N_BYTES: u8 = u16::N_BYTES * 3;
    const N_WORDS: u16 = 3;
}

impl Decode for StateOfChargeSettings {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        // Note that the ordering is important here:
        let min_system = Percentage::decode_from(buf)?;
        let max = Percentage::decode_from(buf)?;
        let min_on_grid = Percentage::decode_from(buf)?;

        Ok(Self { min_system, min_on_grid, max })
    }
}
