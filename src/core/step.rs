use crate::{
    core::working_mode::WorkingMode,
    units::{currency::Cost, energy::KilowattHours},
};

/// Single-hour working plan step.
#[derive(Copy, Clone)]
pub struct Step {
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_consumption: KilowattHours,
    pub loss: Cost,
}
