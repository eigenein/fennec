use crate::quantity::{
    energy::{DecawattHours, WattHours},
    power::Watts,
    ratios::Percentage,
};

#[must_use]
pub struct State {
    /// State-of-charge (SoC) percentage.
    pub charge: Percentage,

    /// State-of-health (SoH) percentage.
    pub health: Percentage,

    /// Design capacity – constant for the product lifetime.
    pub design_capacity: DecawattHours,

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
}
