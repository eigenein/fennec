use fennec_quantities::{
    cost::Cost,
    energy::KilowattHours,
    power::Kilowatts,
    rate::KilowattHourRate,
};

use crate::core::{interval::Interval, working_mode::WorkingMode};

/// Single-hour working plan step.
///
/// Technically, it is not needed to store all the attributes here because I could always zip
/// the back track with the original metrics, but having it here makes it much easier to work with.
#[derive(Clone)]
pub struct Step {
    /// Loss within this single step.
    pub loss: Cost,

    pub interval: Interval,
    pub grid_rate: KilowattHourRate,
    pub stand_by_power: Kilowatts,
    pub working_mode: WorkingMode,
    pub residual_energy_before: KilowattHours,
    pub residual_energy_after: KilowattHours,
    pub grid_consumption: KilowattHours,
}

impl Step {
    pub fn residual_energy_change(&self) -> KilowattHours {
        self.residual_energy_after - self.residual_energy_before
    }

    pub fn charge(&self) -> KilowattHours {
        self.residual_energy_change().max(KilowattHours::ZERO)
    }

    pub fn discharge(&self) -> KilowattHours {
        -self.residual_energy_change().min(KilowattHours::ZERO)
    }
}
