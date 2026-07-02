//! Shortcuts for MiniQube functions.
//!
//! # Example
//!
//! ```rust,no_run
//! use anyhow::Result;
//! use fennec_modbus::{
//!     contrib::mini_qube,
//!     protocol::address,
//!     tcp::{UnitId, tokio::Client},
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let state_of_charge = Client::new("battery.iot.home.arpa:502")
//!         .call::<mini_qube::functions::ReadStateOfCharge>(mini_qube::UNIT_ID, address::Const)
//!         .await?;
//!     Ok(())
//! }
//! ```

use super::{schedule, types};
use crate::{
    contrib::types::{DecawattHours, Percentage, Watts},
    protocol::{
        address,
        function::{ReadHoldingRegisters, ReadWriteRegisters, WriteMultipleRegisters},
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

/// Read the battery total energy exported to grid.
pub type ReadTotalGridExportEnergy =
    ReadHoldingRegisters<address::Const<39621>, DecawattHours<u32>>;

/// Read the battery total energy imported from grid.
pub type ReadTotalGridImportEnergy =
    ReadHoldingRegisters<address::Const<39625>, DecawattHours<u32>>;

/// Read the state-of-charge settings in a single transaction.
///
/// Reading three registers in one transaction reduces network latency and ensures atomic consistency
/// compared to three separate read operations.
pub type ReadStateOfChargeSettings =
    ReadHoldingRegisters<address::Const<46609>, types::StateOfChargeSettings>;

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
///
/// If you're reading the complete schedule, consider calling [`ReadScheduleEntryBlock`] instead.
pub type ReadScheduleEntry =
    ReadHoldingRegisters<address::Stride<48010, schedule::Slot>, schedule::Slot>;

/// Read 12 schedule entries at a time.
pub type ReadScheduleEntryBlock = ReadHoldingRegisters<schedule::BlockIndex, schedule::Block>;

/// Write schedule entry.
///
/// This function accepts the slot index as the argument.
///
/// If you're writing the complete schedule, consider calling [`WriteScheduleEntryBlock`] instead.
pub type WriteScheduleEntry =
    WriteMultipleRegisters<address::Stride<48010, schedule::Slot>, schedule::Slot>;

/// Write 12 schedule entries at a time.
pub type WriteScheduleEntryBlock = WriteMultipleRegisters<schedule::BlockIndex, schedule::Block>;

/// Write and read 12 schedule entries at a time.
///
/// Note: Fox ESS MQ2200 returns "illegal function" for this one.
pub type ReadWriteScheduleEntryBlock = ReadWriteRegisters<
    schedule::BlockIndex,
    schedule::Block,
    schedule::BlockIndex,
    schedule::Block,
>;
