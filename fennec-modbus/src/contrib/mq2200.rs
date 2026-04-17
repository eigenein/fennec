//! Calls for Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.

use bytes::{Buf, BufMut};

use crate::{
    Error,
    contrib::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        codec::{BigEndian, BitSize, Decoder, Encoder, NativeEndian},
        function::{Read, read::HoldingRegisters},
    },
};

/// Read the battery state-of-health.
pub type ReadStateOfHealth =
    Read<HoldingRegisters, address::Const<37624>, Percentage<u16>, NativeEndian>;

/// Read the battery design capacity.
pub type ReadDesignCapacity =
    Read<HoldingRegisters, address::Const<37635>, DecawattHours<u16>, NativeEndian>;

/// Read the battery total active power (including EPS).
pub type ReadTotalActivePower =
    Read<HoldingRegisters, address::Const<39134>, Watts<i32>, BigEndian>;

/// Read the battery Emergency Power Supply active power.
pub type ReadEpsActivePower = Read<HoldingRegisters, address::Const<39216>, Watts<i32>, BigEndian>;

/// Read the battery state-of-charge.
pub type ReadStateOfCharge =
    Read<HoldingRegisters, address::Const<39424>, Percentage<u16>, NativeEndian>;

/// Read the system minimum allowed state-of-charge.
///
/// Unlike the reserve state-of-charge, this an absolute minimum for any battery state.
pub type ReadMinimumSystemStateOfCharge =
    Read<HoldingRegisters, address::Const<46609>, Percentage<u16>, NativeEndian>;

/// Read maximum allowed state-of-charge.
pub type ReadMaximumStateOfCharge =
    Read<HoldingRegisters, address::Const<46610>, Percentage<u16>, NativeEndian>;

/// Read the minimum allowed state-of-charge in the on-grid mode.
///
/// This is also known as reserve state-of-charge.
pub type ReadMinimumStateOfChargeOnGrid =
    Read<HoldingRegisters, address::Const<46611>, Percentage<u16>, NativeEndian>;

/// Read schedule entry.
///
/// This function accepts the slot index as the argument.
pub type ReadScheduleEntry =
    Read<HoldingRegisters, address::Stride<48010, ScheduleEntry>, ScheduleEntry, NativeEndian>;

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

impl Encoder<WorkingMode> for NativeEndian {
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

impl Decoder<WorkingMode> for NativeEndian {
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

#[derive(Debug)]
pub struct ScheduleEntry {
    pub is_enabled: bool,
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    pub working_mode: WorkingMode,
    pub max_soc: Percentage<u8>,
    pub min_soc: Percentage<u8>,
    pub feed_soc: Percentage<u16>,
    pub power: Watts<u16>,
}

impl BitSize for ScheduleEntry {
    const N_BITS: u16 = 20 * 8;
}

impl Encoder<ScheduleEntry> for NativeEndian {
    fn encode(entry: &ScheduleEntry, to: &mut impl BufMut) {
        to.put_u16(u16::from(entry.is_enabled));
        Self::encode(&entry.start_hour, to);
        Self::encode(&entry.start_minute, to);
        Self::encode(&entry.end_hour, to);
        Self::encode(&entry.end_minute, to);
        Self::encode(&entry.working_mode, to);
        Self::encode(&entry.max_soc, to);
        Self::encode(&entry.min_soc, to);
        Self::encode(&entry.feed_soc, to);
        Self::encode(&entry.power, to);

        // Reserved:
        to.put_u16(0);
        to.put_u16(0);
        to.put_u16(0);
    }
}

impl Decoder<ScheduleEntry> for NativeEndian {
    fn decode(from: &mut impl Buf) -> Result<ScheduleEntry, Error> {
        // Note, 3 words are ignored as reserved.
        Ok(ScheduleEntry {
            is_enabled: from.try_get_u16()? != 0,
            start_hour: Self::decode(from)?,
            start_minute: Self::decode(from)?,
            end_hour: Self::decode(from)?,
            end_minute: Self::decode(from)?,
            working_mode: Self::decode(from)?,
            max_soc: Self::decode(from)?,
            min_soc: Self::decode(from)?,
            feed_soc: Self::decode(from)?,
            power: Self::decode(from)?,
        })
    }
}
