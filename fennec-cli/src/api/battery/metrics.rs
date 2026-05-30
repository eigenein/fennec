use chrono::{DateTime, Local};
use musli::{Decode, Encode};

use crate::{
    energy::Flow,
    quantity::{
        energy::{DecawattHours, MilliwattHours, WattHours},
        power::Watts,
        ratios::Percentage,
    },
};

#[must_use]
#[derive(Encode, Decode)]
pub struct Metrics {
    /// Timestamp of the readings.
    #[musli(Binary, name = 1)]
    #[musli(with = crate::ops::musli::chrono)]
    pub timestamp: DateTime<Local>,

    /// State-of-charge (SoC) percentage.
    #[musli(Binary, name = 2)]
    pub charge: Percentage,

    /// State-of-health (SoH) percentage.
    #[musli(Binary, name = 3)]
    pub health: Percentage,

    /// Design capacity – constant for the product lifetime.
    #[musli(Binary, name = 4)]
    pub design_capacity: DecawattHours,

    /// Battery external active power.
    ///
    /// Positive means discharging, negative means charging.
    #[musli(Binary, name = 5)]
    pub active_power: Watts,

    /// Active power on the EPS output.
    #[musli(Binary, name = 6)]
    pub eps_active_power: Watts,

    #[musli(Binary, name = 7)]
    pub total_grid_flow: Flow<DecawattHours>,
}

impl Metrics {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        WattHours::from(self.design_capacity) * self.health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_energy(&self) -> MilliwattHours {
        self.design_capacity * (self.health * self.charge)
    }
}
