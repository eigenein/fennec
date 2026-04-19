use bytes::{Buf, BufMut};

use crate::{
    Error,
    contrib::{Percentage, Watts},
    protocol::{
        Address,
        address,
        codec::{BitSize, Decode, Encode},
    },
};

/// Schedule block consisting of 12 entries.
pub type EntryBlock = [Entry; 12];

/// Block index for batch-reading 12 schedule entries at a time.
///
/// There are 8 blocks (indices 0–7), covering all 96 entries.
pub struct BlockIndex(pub u16);

impl BlockIndex {
    pub const MAX: u16 = 8;
}

impl Address for BlockIndex {}

impl Encode for BlockIndex {
    fn encode(&self, to: &mut impl BufMut) {
        address::Stride::<48010, EntryBlock>::from(self.0).encode(to);
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(u16)]
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
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(match self {
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
    fn decode(from: &mut impl Buf) -> Result<Self, Error> {
        Ok(match from.try_get_u16()? {
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

/// Scheduler entry start or end time.
#[derive(Copy, Clone, Debug)]
pub struct NaiveTime {
    pub hour: u8,
    pub minute: u8,
}

impl NaiveTime {
    pub const MIN: Self = Self { hour: 0, minute: 0 };

    /// The last minute of a day is always _inclusive_.
    pub const MAX: Self = Self { hour: 23, minute: 59 };
}

impl Encode for NaiveTime {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u8(self.hour);
        to.put_u8(self.minute);
    }
}

impl Decode for NaiveTime {
    fn decode(from: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self { hour: from.try_get_u8()?, minute: from.try_get_u8()? })
    }
}

/// Mode scheduler entry.
#[derive(Copy, Clone, Debug)]
pub struct Entry {
    pub is_enabled: bool,

    /// Time slot start time, inclusive.
    pub start_time: NaiveTime,

    /// Time slot end time, exclusive.
    ///
    /// Note that 23:59 is special as it is *inclusive*. 00:00 cannot be set as end time.
    /// Confirmed with Fox ESS support that this the intended behaviour.
    pub end_time: NaiveTime,

    pub working_mode: WorkingMode,
    pub maximum_state_of_charge: Percentage<u8>,
    pub minimum_state_of_charge: Percentage<u8>,

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

impl Entry {
    /// Total number of schedule entries in the register space.
    pub const COUNT: u16 = 96;

    /// Disabled entry.
    ///
    /// Actual contents _should not_ matter, but set to safe fallback default.
    pub const DISABLED: Self = Self {
        is_enabled: false,
        start_time: NaiveTime { hour: 0, minute: 0 },
        end_time: NaiveTime { hour: 0, minute: 0 },
        working_mode: WorkingMode::SelfUse,
        maximum_state_of_charge: Percentage(100),
        minimum_state_of_charge: Percentage(10),
        target_state_of_charge: Percentage(100),
        power: Watts(0),
        reserved_1: 0,
        reserved_2: 0,
        reserved_3: 0,
    };
}

impl BitSize for Entry {
    const N_BITS: u16 = 20 * 8;
}

impl Encode for Entry {
    fn encode(&self, to: &mut impl BufMut) {
        to.put_u16(u16::from(self.is_enabled));
        self.start_time.encode(to);
        self.end_time.encode(to);
        self.working_mode.encode(to);
        to.put_u8(self.maximum_state_of_charge.0);
        to.put_u8(self.minimum_state_of_charge.0);
        self.target_state_of_charge.encode(to);
        self.power.encode(to);
        self.reserved_1.encode(to);
        self.reserved_2.encode(to);
        self.reserved_3.encode(to);
    }
}

impl Decode for Entry {
    fn decode(from: &mut impl Buf) -> Result<Self, Error> {
        Ok(Self {
            is_enabled: from.try_get_u16()? != 0,
            start_time: NaiveTime::decode(from)?,
            end_time: NaiveTime::decode(from)?,
            working_mode: WorkingMode::decode(from)?,
            maximum_state_of_charge: Percentage(from.try_get_u8()?),
            minimum_state_of_charge: Percentage(from.try_get_u8()?),
            target_state_of_charge: Percentage::decode(from)?,
            power: Watts::decode(from)?,
            reserved_1: u16::decode(from)?,
            reserved_2: u16::decode(from)?,
            reserved_3: u16::decode(from)?,
        })
    }
}
