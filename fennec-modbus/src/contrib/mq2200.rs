//! Calls for Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.

use bytes::{Buf, BufMut};

use crate::{
    Error,
    contrib::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        codec::{BigEndian, BitSize, Decoder, Encoder, Struct, Word},
        function::ReadHoldingRegisters,
    },
};

/// Read the battery state-of-health.
pub type ReadStateOfHealth = ReadHoldingRegisters<address::Const<37624>, Percentage<u16>, Word>;

/// Read the battery design capacity.
pub type ReadDesignCapacity = ReadHoldingRegisters<address::Const<37635>, DecawattHours<u16>, Word>;

/// Read the battery total active power (including EPS).
pub type ReadTotalActivePower = ReadHoldingRegisters<address::Const<39134>, Watts<i32>, BigEndian>;

/// Read the battery Emergency Power Supply active power.
pub type ReadEpsActivePower = ReadHoldingRegisters<address::Const<39216>, Watts<i32>, BigEndian>;

/// Read the battery state-of-charge.
pub type ReadStateOfCharge = ReadHoldingRegisters<address::Const<39424>, Percentage<u16>, Word>;

/// Read the system minimum allowed state-of-charge.
///
/// Unlike the reserve state-of-charge, this an absolute minimum for any battery state.
pub type ReadMinimumSystemStateOfCharge =
    ReadHoldingRegisters<address::Const<46609>, Percentage<u16>, Word>;

/// Read maximum allowed state-of-charge.
pub type ReadMaximumStateOfCharge =
    ReadHoldingRegisters<address::Const<46610>, Percentage<u16>, Word>;

/// Read the minimum allowed state-of-charge in the on-grid mode.
///
/// This is also known as reserve state-of-charge.
pub type ReadMinimumStateOfChargeOnGrid =
    ReadHoldingRegisters<address::Const<46611>, Percentage<u16>, Word>;

/// Read schedule entry.
///
/// This function accepts the slot index as the argument.
pub type ReadScheduleEntry =
    ReadHoldingRegisters<address::Stride<48010, ScheduleEntry>, ScheduleEntry, Struct>;

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

impl Encoder<WorkingMode> for Word {
    fn encode(value: &WorkingMode, to: &mut impl BufMut) {
        to.put_u16(match value {
            WorkingMode::SelfUse => 1,
            WorkingMode::FeedInPriority => 2,
            WorkingMode::BackUp => 3,
            WorkingMode::PeakShaving => 4,
            WorkingMode::ForceCharge => 6,
            WorkingMode::ForceDischarge => 7,
            WorkingMode::Unknown(working_mode) => *working_mode,
        });
    }
}

impl Decoder<WorkingMode> for Word {
    fn decode(from: &mut impl Buf) -> Result<WorkingMode, Error> {
        Ok(match from.try_get_u16()? {
            1 => WorkingMode::SelfUse,
            2 => WorkingMode::FeedInPriority,
            3 => WorkingMode::BackUp,
            4 => WorkingMode::PeakShaving,
            6 => WorkingMode::ForceCharge,
            7 => WorkingMode::ForceDischarge,
            working_mode => WorkingMode::Unknown(working_mode),
        })
    }
}

/// Scheduler entry start or end time.
#[derive(Copy, Clone, Debug)]
pub struct NaiveTime {
    pub hour: u8,
    pub minute: u8,
}

impl Encoder<NaiveTime> for Word {
    fn encode(value: &NaiveTime, to: &mut impl BufMut) {
        to.put_u8(value.hour);
        to.put_u8(value.minute);
    }
}

impl Decoder<NaiveTime> for Word {
    fn decode(from: &mut impl Buf) -> Result<NaiveTime, Error> {
        Ok(NaiveTime { hour: from.try_get_u8()?, minute: from.try_get_u8()? })
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

impl Encoder<ScheduleEntry> for Struct {
    fn encode(entry: &ScheduleEntry, to: &mut impl BufMut) {
        to.put_u16(u16::from(entry.is_enabled));
        Word::encode(&entry.start_time, to);
        Word::encode(&entry.end_time, to);
        Word::encode(&entry.working_mode, to);
        to.put_u8(entry.maximum_state_of_charge.0);
        to.put_u8(entry.minimum_state_of_charge.0);
        Word::encode(&entry.target_state_of_charge, to);
        Word::encode(&entry.power, to);
        Word::encode(&entry.reserved_1, to);
        Word::encode(&entry.reserved_2, to);
        Word::encode(&entry.reserved_3, to);
    }
}

impl Decoder<ScheduleEntry> for Struct {
    fn decode(from: &mut impl Buf) -> Result<ScheduleEntry, Error> {
        Ok(ScheduleEntry {
            is_enabled: from.try_get_u16()? != 0,
            start_time: Word::decode(from)?,
            end_time: Word::decode(from)?,
            working_mode: Word::decode(from)?,
            maximum_state_of_charge: Percentage(from.try_get_u8()?),
            minimum_state_of_charge: Percentage(from.try_get_u8()?),
            target_state_of_charge: Word::decode(from)?,
            power: Word::decode(from)?,
            reserved_1: Word::decode(from)?,
            reserved_2: Word::decode(from)?,
            reserved_3: Word::decode(from)?,
        })
    }
}
