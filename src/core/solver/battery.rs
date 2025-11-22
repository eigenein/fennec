use std::cmp::Ordering;

use chrono::TimeDelta;

use crate::{
    quantity::{energy::KilowattHours, power::Kilowatts},
    statistics::BatteryParameters,
};

/// Battery simulator.
#[derive(Copy, Clone, bon::Builder)]
pub struct Battery {
    capacity: KilowattHours,

    /// Minimally allowed residual energy.
    ///
    /// This is normally calculated from the capacity and minimal state-of-charge setting.
    min_residual_energy: KilowattHours,

    /// Current residual energy.
    residual_energy: KilowattHours,

    parameters: BatteryParameters,
}

impl Battery {
    pub const fn residual_energy(&self) -> KilowattHours {
        self.residual_energy
    }

    /// Apply the requested power and calculate the new state.
    ///
    /// # Returns
    ///
    /// Battery active time.
    #[must_use]
    pub fn apply_load(&mut self, power: Kilowatts, for_: TimeDelta) -> TimeDelta {
        let active_time = self.apply_active_load(power, for_);

        // FIXME: technically, I should also take the parasitic load into account when calculating the active time:
        self.apply_parasitic_load(for_);

        // This will be used to calculate the loss:
        active_time
    }

    #[must_use]
    fn apply_active_load(&mut self, external_power: Kilowatts, for_: TimeDelta) -> TimeDelta {
        let initial_residual_energy = self.residual_energy;

        // Calculate the internal power:
        let internal_power = external_power
            * match external_power.cmp(&Kilowatts::ZERO) {
                Ordering::Greater => self.parameters.charging_efficiency,
                Ordering::Less => 1.0 / self.parameters.discharging_efficiency,
                Ordering::Equal => {
                    return TimeDelta::zero();
                }
            };

        // Update the residual energy:
        self.residual_energy = (self.residual_energy + internal_power * for_).clamp(
            // At the bottom, it's capped by the minimum SoC or residual energy – when it's already lower:
            self.min_residual_energy.min(initial_residual_energy),
            // At the top, it's capped by the capacity or residual energy – when it's somehow higher:
            self.capacity.max(initial_residual_energy),
        );

        // The energy differential and internal power must have the same sign here:
        let active_time = (self.residual_energy - initial_residual_energy) / internal_power;

        assert!(active_time >= TimeDelta::zero());
        active_time
    }

    fn apply_parasitic_load(&mut self, for_: TimeDelta) {
        self.residual_energy =
            (self.residual_energy - self.parameters.parasitic_load * for_).max(KilowattHours::ZERO);
    }
}
