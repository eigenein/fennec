use binrw::{BinRead, BinWrite};

#[derive(Copy, Clone, Debug, Eq, PartialEq, BinRead, BinWrite)]
pub enum UnitId {
    /// Broadcast on a subnetwork. Also accepted for a direct connection.
    #[brw(magic(0_u8))]
    Broadcast,

    /// Direct connection.
    #[brw(magic(255_u8))]
    NonSignificant,

    #[bw(assert(matches!(self_0, 1..=247), "unit ID {self_0} is reserved"))]
    Significant(u8),
}
