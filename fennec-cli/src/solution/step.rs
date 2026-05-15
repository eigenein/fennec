use crate::{
    battery::WorkingMode,
    energy,
    ops::Interval,
    quantity::{energy::WattHours, price::KilowattHourPrice, time::Hours},
    solution,
};

/// Working plan for a single [`crate::ops::Interval`].
#[derive(Copy, Clone)]
pub struct Step {
    pub interval: Interval,
    pub duration: Hours,
    pub energy_price: energy::Flow<KilowattHourPrice>,
    pub energy_balance: energy::Balance<WattHours>,
    pub working_mode: WorkingMode,
    pub residual_energy_after: WattHours,
    pub energy_level_after: usize,
    pub metrics: solution::Metrics,
}
