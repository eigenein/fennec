use std::range::RangeInclusive;

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
pub struct Metrics {
    pub tracked: Tracked,
    pub untracked: Untracked,
}

impl Metrics {
    /// Minimum allowed residual charge.
    pub fn min_residual_charge(&self) -> WattHours {
        self.tracked.design_capacity.rescale() * self.untracked.allowed_charge.start
    }

    /// Maximum allowed residual charge.
    pub fn max_residual_charge(&self) -> WattHours {
        self.tracked.design_capacity.rescale() * self.untracked.allowed_charge.last
    }

    pub fn allowed_energy_levels(&self) -> RangeInclusive<WattHours<usize>> {
        (self.min_residual_charge().into()..=self.max_residual_charge().into()).into()
    }
}

/// Untracked metrics are throw away directly after processing.
#[must_use]
pub struct Untracked {
    /// Allowed state-of-charge.
    pub allowed_charge: RangeInclusive<Percentage>,

    /// Battery external active power.
    ///
    /// Positive means discharging, negative means charging.
    pub active_power: Watts,

    /// Active power on the EPS output.
    pub eps_active_power: Watts,
}

/// Tracked metrics are persisted in order to estimate the battery parameters.
#[must_use]
#[derive(Copy, Clone, Encode, Decode)]
pub struct Tracked {
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

    #[musli(Binary, name = 4)]
    pub design_capacity: DecawattHours,

    #[musli(Binary, name = 7)]
    pub total_grid_flow: Flow<DecawattHours>,
}

impl Tracked {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        self.design_capacity.rescale() * self.health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_energy(&self) -> MilliwattHours {
        self.design_capacity * (self.health * self.charge)
    }
}
