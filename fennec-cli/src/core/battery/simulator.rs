use std::cmp::Ordering;

use chrono::TimeDelta;

use crate::{
    quantity::{energy::KilowattHours, power::Kilowatts},
    statistics::battery::BatteryEfficiency,
};

#[derive(Copy, Clone, bon::Builder)]
pub struct Simulator {
    /// Minimally allowed residual energy.
    ///
    /// This is normally calculated from the capacity and minimal state-of-charge setting.
    min_residual_energy: KilowattHours,

    /// Current residual energy.
    residual_energy: KilowattHours,

    max_residual_energy: KilowattHours,

    efficiency: BatteryEfficiency,
}

impl Simulator {
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
        // FIXME: technically, I should also take the parasitic load into account when calculating the active time:
        self.apply_parasitic_load(for_);

        // This will be used to calculate the loss:
        self.apply_active_load(power, for_)
    }

    #[must_use]
    fn apply_active_load(&mut self, external_power: Kilowatts, for_: TimeDelta) -> TimeDelta {
        let initial_residual_energy = self.residual_energy;

        // Calculate the internal power:
        let internal_power = external_power
            * match external_power.cmp(&Kilowatts::ZERO) {
                Ordering::Greater => self.efficiency.charging,
                Ordering::Less => 1.0 / self.efficiency.discharging,
                Ordering::Equal => {
                    return TimeDelta::zero();
                }
            };

        // Update the residual energy:
        self.residual_energy = (self.residual_energy + internal_power * for_).clamp(
            // At the bottom, it's capped by the minimum SoC or residual energy – whatever is lower:
            self.min_residual_energy.min(initial_residual_energy),
            // At the top, it's capped by the capacity or residual energy – whatever is higher:
            self.max_residual_energy.max(initial_residual_energy),
        );

        // The energy differential and internal power must have the same sign here:
        let active_time = (self.residual_energy - initial_residual_energy) / internal_power;

        assert!(active_time >= TimeDelta::zero());
        active_time
    }

    fn apply_parasitic_load(&mut self, for_: TimeDelta) {
        self.residual_energy =
            (self.residual_energy - self.efficiency.parasitic_load * for_).max(KilowattHours::ZERO);
    }
}
