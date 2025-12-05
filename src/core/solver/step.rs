use crate::{
    core::working_mode::WorkingMode,
    quantity::{
        cost::Cost,
        energy::KilowattHours,
        interval::Interval,
        power::Kilowatts,
        rate::KilowattHourRate,
    },
};

/// Single-hour working plan step.
#[derive(Clone)]
pub struct Step {
    /// Technically, it is not needed to store the timestamp here because I could always zip
    /// the back track with the original metrics, but having it here makes it much easier to work with
    /// (and to ensure it is working properly).
    pub interval: Interval,

    pub grid_rate: KilowattHourRate,
    pub stand_by_power: Kilowatts,
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_consumption: KilowattHours,
    pub loss: Cost,
}
