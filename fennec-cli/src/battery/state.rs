use crate::{
    ops::range,
    quantity::{
        energy::{DecawattHours, WattHours},
        power::Watts,
        ratios::Percentage,
    },
};

#[must_use]
pub struct State {
    /// State-of-charge (SoC) percentage.
    pub charge: Percentage,

    /// State-of-health (SoH) percentage.
    pub health: Percentage,

    /// Design capacity – constant for the product lifetime.
    pub design_capacity: DecawattHours,

    /// Allowed on-grid SoC levels.
    pub charge_range: range::Inclusive<Percentage>,

    /// Global system SoC minimum.
    pub min_system_charge: Percentage,

    /// Battery external active power.
    ///
    /// Positive means discharging, negative means charging.
    pub active_power: Watts,

    /// Active power on the EPS output.
    pub eps_active_power: Watts,
}

impl State {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        WattHours::from(self.design_capacity) * self.health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.charge
    }

    pub fn min_residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.charge_range.min
    }

    pub fn max_residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.charge_range.max
    }
}
