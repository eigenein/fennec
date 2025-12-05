use crate::{
    core::working_mode::WorkingMode,
    quantity::{cost::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

/// Single-hour working plan step.
#[derive(Clone)]
pub struct Step {
    pub grid_rate: KilowattHourRate,
    pub stand_by_power: Kilowatts,
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_consumption: KilowattHours,
    pub loss: Cost,
}
