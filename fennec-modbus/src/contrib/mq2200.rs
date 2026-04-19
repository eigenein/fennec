//! Calls for Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.

use bytes::{Buf, BufMut};

use crate::{
    Error,
    contrib::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        codec::{BitSize, Decode, Encode},
        function::ReadHoldingRegisters,
    },
};

/// Read the battery state-of-health.
pub type ReadStateOfHealth = ReadHoldingRegisters<address::Const<37624>, Percentage<u16>>;

/// Read the battery design capacity.
pub type ReadDesignCapacity = ReadHoldingRegisters<address::Const<37635>, DecawattHours<u16>>;

/// Read the battery total active power (including EPS).
pub type ReadTotalActivePower = ReadHoldingRegisters<address::Const<39134>, Watts<i32>>;

/// Read the battery Emergency Power Supply active power.
pub type ReadEpsActivePower = ReadHoldingRegisters<address::Const<39216>, Watts<i32>>;

/// Read the battery state-of-charge.
pub type ReadStateOfCharge = ReadHoldingRegisters<address::Const<39424>, Percentage<u16>>;

/// Read the system minimum allowed state-of-charge.
///
/// Unlike the reserve state-of-charge, this an absolute minimum for any battery state.
pub type ReadMinimumSystemStateOfCharge =
    ReadHoldingRegisters<address::Const<46609>, Percentage<u16>>;

/// Read maximum allowed state-of-charge.
pub type ReadMaximumStateOfCharge = ReadHoldingRegisters<address::Const<46610>, Percentage<u16>>;

/// Read the minimum allowed state-of-charge in the on-grid mode.
///
/// This is also known as reserve state-of-charge.
pub type ReadMinimumStateOfChargeOnGrid =
    ReadHoldingRegisters<address::Const<46611>, Percentage<u16>>;

/// Read schedule entry.
///
/// This function accepts the slot index as the argument.
pub type ReadScheduleEntry =
    ReadHoldingRegisters<address::Stride<48010, ScheduleEntry>, ScheduleEntry>;

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
#[derive(Debug)]
pub struct ScheduleEntry {
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

impl BitSize for ScheduleEntry {
    const N_BITS: u16 = 20 * 8;
}

impl Encode for ScheduleEntry {
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

impl Decode for ScheduleEntry {
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
