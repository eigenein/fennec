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
//!         .call::<mini_qube::ReadStateOfCharge>(mini_qube::UNIT_ID, address::Const)
//!         .await?;
//!     Ok(())
//! }
//! ```

use super::schedule;
use crate::protocol::{
    address,
    function::{ReadHoldingRegisters, ReadWriteRegisters, WriteMultipleRegisters},
};

/// Read schedule slot.
///
/// This function accepts the slot index as the argument.
///
/// If you're reading the complete schedule, consider calling [`ReadScheduleSlotBlock`] instead.
pub type ReadScheduleEntry = ReadHoldingRegisters<
    address::Stride<48010, { schedule::Slot::N_TOTAL }, schedule::Slot>,
    schedule::Slot,
>;

/// Read 12 schedule slots at a time.
pub type ReadScheduleSlotBlock = ReadHoldingRegisters<schedule::BlockIndex, schedule::Block>;

/// Write schedule slot.
///
/// This function accepts the slot index as the argument.
///
/// If you're writing the complete schedule, consider calling [`WriteScheduleSlotBlock`] instead.
pub type WriteScheduleSlot = WriteMultipleRegisters<
    address::Stride<48010, { schedule::Slot::N_TOTAL }, schedule::Slot>,
    schedule::Slot,
>;

/// Write 12 schedule entries at a time.
pub type WriteScheduleSlotBlock = WriteMultipleRegisters<schedule::BlockIndex, schedule::Block>;

/// Write and read 12 schedule entries at a time.
///
/// Note: Fox ESS MQ2200 returns "illegal function" with incorrect function code for this one.
pub type ReadWriteScheduleSlotBlock = ReadWriteRegisters<
    schedule::BlockIndex,
    schedule::Block,
    schedule::BlockIndex,
    schedule::Block,
>;
