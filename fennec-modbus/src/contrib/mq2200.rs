//! Calls for Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.

use crate::{
    contrib::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        codec::{BigEndian, NativeEndian},
        function::{Read, read::HoldingRegisters},
    },
};

/// Read the battery state-of-health.
pub type ReadStateOfHealth =
    Read<HoldingRegisters, address::Const<37624>, Percentage, NativeEndian>;

/// Read the battery design capacity.
pub type ReadDesignCapacity =
    Read<HoldingRegisters, address::Const<37635>, DecawattHours, NativeEndian>;

/// Read the battery total active power (including EPS).
pub type ReadTotalActivePower = Read<HoldingRegisters, address::Const<39134>, Watts, BigEndian>;

/// Read the battery Emergency Power Supply active power.
pub type ReadEpsActivePower = Read<HoldingRegisters, address::Const<39216>, Watts, BigEndian>;

/// Read the battery state-of-charge.
pub type ReadStateOfCharge =
    Read<HoldingRegisters, address::Const<39424>, Percentage, NativeEndian>;

/// Read the system minimum allowed state-of-charge.
///
/// Unlike the reserve state-of-charge, this an absolute minimum for any battery state.
pub type ReadMinimumSystemStateOfCharge =
    Read<HoldingRegisters, address::Const<46609>, Percentage, NativeEndian>;

/// Read maximum allowed state-of-charge.
pub type ReadMaximumStateOfCharge =
    Read<HoldingRegisters, address::Const<46610>, Percentage, NativeEndian>;

/// Read the minimum allowed state-of-charge in the on-grid mode.
///
/// This is also known as reserve state-of-charge.
pub type ReadMinimumStateOfChargeOnGrid =
    Read<HoldingRegisters, address::Const<46611>, Percentage, NativeEndian>;
