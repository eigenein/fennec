use enumset::EnumSet;

use crate::{
    core::working_mode::WorkingMode,
    quantity::{power::Kilowatts, rate::KilowattHourRate},
};

/// External, usually hourly, conditions for the solver.
#[derive(Copy, Clone)]
pub struct Conditions {
    pub grid_rate: KilowattHourRate,

    pub allowed_working_modes: EnumSet<WorkingMode>,

    /// Net power consumption by the household (negative is solar power excess).
    pub stand_by_power: Kilowatts,
}
