use core::fmt::{Display, Formatter};

use bytes::{Buf, BufMut};

use crate::{
    Error,
    contrib::types::{Percentage, Watts},
    protocol::{
        Address,
        address,
        codec::{BitSize, Decode, Encode},
    },
};

/// Stride of schedule slot blocks.
///
/// There are [`Slot::N_TOTAL`] schedule slots starting from here.
pub type BlockStride = address::Stride<48010, Block>;

/// Number of slots per schedule block.
///
/// There are [`N_BLOCKS`] such blocks.
pub const N_SLOTS_PER_BLOCK: usize = 12;

/// Number of schedule blocks, each consisting of [`N_SLOTS_PER_BLOCK`] slots.
pub const N_BLOCKS: usize = 8;

/// Type alias for a full schedule of [`Slot::N_TOTAL`] slots.
///
/// Note that this is not encodable nor decodable as it doesn't fit the Modbus payload size.
/// The type alias is provided solely for convenience.
pub type Full = [Slot; Slot::N_TOTAL];

/// Schedule block consisting of [`N_SLOTS_PER_BLOCK`] slots.
pub type Block = [Slot; N_SLOTS_PER_BLOCK];

/// Block index for batch-reading [`N_SLOTS_PER_BLOCK`] schedule slots at a time.
///
/// There are [`N_BLOCKS`] blocks (indices 0–7), covering all [`Slot::N_TOTAL`] slots.
#[must_use]
#[derive(Copy, Clone)]
pub struct BlockIndex(pub u16);

impl BlockIndex {
    /// Last valid schedule block index.
    #[expect(clippy::cast_possible_truncation)]
    pub const LAST: u16 = (N_BLOCKS - 1) as u16;
}

impl Address for BlockIndex {}

impl Encode for BlockIndex {
    fn encode_to(&self, buf: &mut impl BufMut) {
        BlockStride::new(self.0).encode_to(buf);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u16)]
#[must_use]
pub enum WorkingMode {
    SelfUse = 1_u16,
    FeedInPriority = 2_u16,
    BackUp = 3_u16,
    PeakShaving = 4_u16,
    ForceCharge = 6_u16,
    ForceDischarge = 7_u16,
    Unknown(u16),
}

impl Encode for WorkingMode {
    fn encode_to(&self, buf: &mut impl BufMut) {
        buf.put_u16(match self {
            Self::SelfUse => 1,
            Self::FeedInPriority => 2,
            Self::BackUp => 3,
            Self::PeakShaving => 4,
            Self::ForceCharge => 6,
            Self::ForceDischarge => 7,
            Self::Unknown(working_mode) => *working_mode,
        });
    }
}

impl Decode for WorkingMode {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(match buf.try_get_u16()? {
            1 => Self::SelfUse,
            2 => Self::FeedInPriority,
            3 => Self::BackUp,
            4 => Self::PeakShaving,
            6 => Self::ForceCharge,
            7 => Self::ForceDischarge,
            working_mode => Self::Unknown(working_mode),
        })
    }
}

/// Scheduler slot start or end time.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct NaiveTime {
    pub hour: u8,
    pub minute: u8,
}

impl Display for NaiveTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl NaiveTime {
    /// The first minute of a day.
    pub const MIN: Self = Self { hour: 0, minute: 0 };

    /// The last minute of a day.
    ///
    /// Note that it is always _inclusive_.
    pub const MAX: Self = Self { hour: 23, minute: 59 };
}

impl Encode for NaiveTime {
    fn encode_to(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.hour);
        buf.put_u8(self.minute);
    }
}

impl Decode for NaiveTime {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self { hour: buf.try_get_u8()?, minute: buf.try_get_u8()? })
    }
}

/// Single schedule slot.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct Slot {
    pub is_enabled: bool,

    /// Time slot start time, inclusive.
    pub start_time: NaiveTime,

    /// Time slot end time, exclusive.
    ///
    /// Note that 23:59 is special as it is *inclusive*. 00:00 cannot be set as end time.
    /// Confirmed with Fox ESS support that this the intended behaviour.
    pub end_time: NaiveTime,

    pub working_mode: WorkingMode,
    pub max_state_of_charge: Percentage<u8>,
    pub min_state_of_charge: Percentage<u8>,

    /// This is called "feed SoC" or "fdSoC", but in reality, it is a target SoC
    /// for charging or discharging.
    #[allow(clippy::doc_markdown)]
    pub target_state_of_charge: Percentage<u16>,

    pub power: Watts<u16>,

    /// Reserved, set to zero.
    pub reserved_1: u16,

    /// Reserved, set to zero.
    pub reserved_2: u16,

    /// Reserved, set to zero.
    pub reserved_3: u16,
}

impl Slot {
    /// Total number of schedule slots in the register space.
    pub const N_TOTAL: usize = N_BLOCKS * N_SLOTS_PER_BLOCK;
}

impl BitSize for Slot {
    const N_BITS: u16 = 20 * 8;
}

impl Encode for Slot {
    fn encode_to(&self, buf: &mut impl BufMut) {
        buf.put_u16(u16::from(self.is_enabled));
        self.start_time.encode_to(buf);
        self.end_time.encode_to(buf);
        self.working_mode.encode_to(buf);
        buf.put_u8(self.max_state_of_charge.0);
        buf.put_u8(self.min_state_of_charge.0);
        self.target_state_of_charge.encode_to(buf);
        self.power.encode_to(buf);
        self.reserved_1.encode_to(buf);
        self.reserved_2.encode_to(buf);
        self.reserved_3.encode_to(buf);
    }
}

impl Decode for Slot {
    fn decode_from(buf: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self {
            is_enabled: buf.try_get_u16()? != 0,
            start_time: NaiveTime::decode_from(buf)?,
            end_time: NaiveTime::decode_from(buf)?,
            working_mode: WorkingMode::decode_from(buf)?,
            max_state_of_charge: Percentage(buf.try_get_u8()?),
            min_state_of_charge: Percentage(buf.try_get_u8()?),
            target_state_of_charge: Percentage::decode_from(buf)?,
            power: Watts::decode_from(buf)?,
            reserved_1: u16::decode_from(buf)?,
            reserved_2: u16::decode_from(buf)?,
            reserved_3: u16::decode_from(buf)?,
        })
    }
}
