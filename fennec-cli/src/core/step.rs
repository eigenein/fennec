use crate::{
    core::{energy_level::EnergyLevel, working_mode::WorkingMode},
    ops::Interval,
    quantity::{currency::Mills, energy::WattHours, rate::KilowattHourRate},
    statistics::flow::SystemFlow,
};

/// Single-hour working plan step.
///
/// Technically, it is not needed to store all the attributes here because I could always zip
/// the back track with the original metrics, but having it here makes it much easier to work with.
pub struct Step {
    /// Loss within this single step.
    pub loss: Mills,

    pub interval: Interval,
    pub grid_rate: KilowattHourRate,
    pub system_flow: SystemFlow<WattHours>,
    pub working_mode: WorkingMode,
    pub residual_energy_after: WattHours,
    pub energy_level_after: EnergyLevel,
}
