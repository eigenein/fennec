use crate::{
    battery::WorkingMode,
    energy,
    quantity::{
        energy::{EnergyLevel, WattHours},
        time::Hours,
    },
    solution,
};

/// Per-interval decision, one step of a [`super::Plan`].
#[must_use]
#[derive(Copy, Clone)]
pub struct Step {
    /// Calculated time interval duration.
    ///
    /// It is normally equal to the original interval duration,
    /// except for first truncated interval.
    pub duration: Hours,

    /// Cumulative energy balance within the time interval.
    pub energy_balance: energy::Balance<WattHours>,

    /// Battery working mode taken by the optimizer.
    pub working_mode: WorkingMode,

    /// Target state at the next stage.
    pub energy_level_after: EnergyLevel,

    /// Stage cost.
    pub metrics: solution::Metrics,
}
