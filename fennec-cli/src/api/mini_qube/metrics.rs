use std::range::RangeInclusive;

use crate::{
    energy::Flow,
    quantity::{
        energy::{DecawattHours, MilliwattHours, WattHours},
        power::Watts,
        ratios::Percentage,
    },
};

#[must_use]
pub struct Metrics {
    /// State-of-charge (SoC) percentage.
    pub state_of_charge: Percentage,

    /// State-of-health (SoH) percentage.
    pub state_of_health: Percentage,

    pub design_capacity: DecawattHours,

    pub total_grid_flow: Flow<DecawattHours>,

    /// Allowed state-of-charge range.
    pub allowed_soc: RangeInclusive<Percentage>,

    /// Battery external active power.
    ///
    /// Positive means discharging, negative means charging.
    pub active_power: Watts,

    /// Active power on the EPS output.
    pub eps_active_power: Watts,
}

impl Metrics {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        self.design_capacity.rescale() * self.state_of_health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_energy(&self) -> MilliwattHours {
        self.design_capacity * (self.state_of_health * self.state_of_charge)
    }

    pub fn allowed_residual_energy(&self) -> RangeInclusive<WattHours<usize>> {
        let actual_capacity = self.actual_capacity();
        let start_energy_level: WattHours<usize> =
            (actual_capacity * self.allowed_soc.start).into();
        let last_energy_level: WattHours<usize> = (actual_capacity * self.allowed_soc.last).into();
        (start_energy_level..=last_energy_level).into()
    }
}
