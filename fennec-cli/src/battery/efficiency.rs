use crate::{
    ops::Integrator,
    quantity::{energy::WattHours, power::Watts},
};

#[derive(Copy, Clone)]
pub struct Efficiency {
    pub charging: f64,
    pub discharging: f64,
    pub parasitic_load: Watts,
}

impl Efficiency {
    pub const fn round_trip(self) -> f64 {
        self.charging * self.discharging
    }
}

#[derive(Copy, Clone)]
pub struct EfficiencyEstimator {
    active_power_integrator: Integrator<WattHours>,
    residual_energy_integrator: Integrator<WattHours>,
}

impl EfficiencyEstimator {
    pub const fn new() -> Self {
        Self {
            active_power_integrator: Integrator::new(),
            residual_energy_integrator: Integrator::new(),
        }
    }

    pub fn push(
        &mut self,
        active_power_sample: Integrator<WattHours>,
        residual_energy_sample: Integrator<WattHours>,
    ) {
        self.active_power_integrator += active_power_sample;
        self.residual_energy_integrator += residual_energy_sample;
    }

    pub fn sub_residual_energy(&mut self, power: Watts) {
        self.residual_energy_integrator.value -= power * self.residual_energy_integrator.duration;
    }

    /// Estimate efficiency of residual energy change to the active power integral.
    ///
    /// Note that for discharging, this will normally be greater than one.
    pub fn estimate(self) -> f64 {
        self.residual_energy_integrator
            .average()
            .zip(self.active_power_integrator.average())
            .map(|(charge, consumption)| charge / consumption)
            .filter(|it| it.is_finite())
            .unwrap_or(1.0)
    }
}
