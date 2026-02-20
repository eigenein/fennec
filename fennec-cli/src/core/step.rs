use crate::{
    core::{energy_level::EnergyLevel, flow::EnergyBalance, solution, working_mode::WorkingMode},
    ops::Interval,
    quantity::{energy::WattHours, price::KilowattHourPrice},
};

/// Single-hour working plan step.
///
/// Technically, it is not needed to store all the attributes here because I could always zip
/// the back track with the original metrics, but having it here makes it much easier to work with.
pub struct Step {
    pub interval: Interval,
    pub energy_price: KilowattHourPrice,
    pub energy_balance: EnergyBalance<WattHours>,
    pub working_mode: WorkingMode,
    pub residual_energy_after: WattHours,
    pub energy_level_after: EnergyLevel,
    pub metrics: solution::Metrics,
}
