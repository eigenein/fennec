use crate::{
    core::{Series, WorkingMode},
    units::{Cost, KilowattHours},
};

/// Optimization plan that describes how the battery will work in the upcoming hours.
pub struct Solution {
    pub net_loss: Cost,
    pub net_loss_without_battery: Cost,
    pub steps: Series<Step>,
}

impl Solution {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}

/// Single-hour working plan step.
#[derive(Copy, Clone)]
pub struct Step {
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_consumption: KilowattHours,
    pub loss: Cost,
}
