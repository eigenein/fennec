use crate::{
    battery::WorkingMode,
    energy,
    quantity::{energy::WattHours, time::Hours},
    solution,
};

/// Working plan for a single [`crate::ops::Interval`].
#[derive(Copy, Clone)]
pub struct Step {
    pub duration: Hours,
    pub energy_balance: energy::Balance<WattHours>,
    pub working_mode: WorkingMode,
    pub residual_energy_after: WattHours,
    pub energy_level_after: usize,
    pub metrics: solution::Metrics,
}
