//! Calls for Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.

pub mod schedule;

use crate::{
    contrib::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        function::{ReadHoldingRegisters, WriteMultipleRegisters},
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
    ReadHoldingRegisters<address::Stride<48010, schedule::Entry>, schedule::Entry>;

/// Read 12 schedule entries at a time.
pub type ReadScheduleEntryBlock = ReadHoldingRegisters<schedule::BlockIndex, schedule::EntryBlock>;

/// Write 12 schedule entries at a time.
pub type WriteScheduleEntryBlock =
    WriteMultipleRegisters<schedule::BlockIndex, schedule::EntryBlock>;
