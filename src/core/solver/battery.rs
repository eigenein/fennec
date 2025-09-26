use crate::quantity::{energy::KilowattHours, power::Kilowatts, time::Hours};

/// Battery simulator.
#[derive(Clone, bon::Builder)]
pub struct Battery {
    capacity: KilowattHours,

    efficiency: f64,

    self_discharge: Kilowatts,

    /// Minimally allowed residual energy.
    ///
    /// This is normally calculated from the capacity and minimal state-of-charge setting.
    min_residual_energy: KilowattHours,

    /// Current residual energy.
    residual_energy: KilowattHours,
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
    pub fn apply_load(&mut self, power: Kilowatts, for_: Hours) -> Hours {
        let initial_residual_energy = self.residual_energy;

        let active_time = if power > Kilowatts::ZERO {
            // While charging, the residual energy grows slower:
            let internal_power = power * self.efficiency;
            self.residual_energy = (self.residual_energy + internal_power * for_)
                .min(self.capacity.max(self.residual_energy));
            let time_charging = (self.residual_energy - initial_residual_energy) / internal_power;
            assert!(time_charging >= Hours::ZERO);
            time_charging
        } else if power < Kilowatts::ZERO {
            // While discharging, the residual energy is spent faster:
            let internal_power = power / self.efficiency;
            // Remember that the power here is negative, hence the `+`:
            self.residual_energy = (self.residual_energy + internal_power * for_)
                .max(self.min_residual_energy.min(initial_residual_energy));
            let time_discharging =
                (self.residual_energy - initial_residual_energy) / internal_power;
            assert!(time_discharging >= Hours::ZERO);
            time_discharging
        } else {
            // Idle:
            Hours::ZERO
        };

        self.apply_self_discharge(for_ - active_time);

        active_time
    }

    fn apply_self_discharge(&mut self, idle_time: Hours) {
        self.residual_energy =
            (self.residual_energy - self.self_discharge * idle_time).max(KilowattHours::ZERO);
    }
}
