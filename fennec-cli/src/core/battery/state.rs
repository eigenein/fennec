use crate::{
    ops::RangeInclusive,
    quantity::{
        energy::{DecawattHours, MilliwattHours, WattHours},
        ratios::Percentage,
    },
};

#[must_use]
pub struct EnergyState {
    pub design_capacity: DecawattHours,
    pub state_of_charge: Percentage,
    pub state_of_health: Percentage,
}

impl EnergyState {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        WattHours::from(self.design_capacity) * self.state_of_health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual(&self) -> WattHours {
        self.actual_capacity() * self.state_of_charge
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_millis(&self) -> MilliwattHours {
        self.design_capacity * (self.state_of_health * self.state_of_charge)
    }
}

#[must_use]
pub struct FullState {
    pub energy: EnergyState,
    pub allowed_state_of_charge: RangeInclusive<Percentage>,
}

impl FullState {
    pub fn min_residual_energy(&self) -> WattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.min
    }

    pub fn max_residual_energy(&self) -> WattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.max
    }
}
