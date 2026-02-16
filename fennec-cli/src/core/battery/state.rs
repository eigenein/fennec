use crate::{
    ops::RangeInclusive,
    quantity::{
        energy::{DecawattHours, KilowattHours, MilliwattHours},
        proportions::Percent,
    },
};

#[must_use]
pub struct EnergyState {
    pub design_capacity: DecawattHours,
    pub state_of_charge: Percent,
    pub state_of_health: Percent,
}

impl EnergyState {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> KilowattHours {
        KilowattHours::from(self.design_capacity) * self.state_of_health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual(&self) -> KilowattHours {
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
    pub allowed_state_of_charge: RangeInclusive<Percent>,
}

impl FullState {
    pub fn min_residual_energy(&self) -> KilowattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.min
    }

    pub fn max_residual_energy(&self) -> KilowattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.max
    }
}
