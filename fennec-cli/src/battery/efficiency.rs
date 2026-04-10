use crate::{
    ops::Integrator,
    quantity::{energy::WattHours, power::Watts, time::Hours},
};

#[must_use]
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

#[must_use]
#[derive(Copy, Clone)]
pub struct EfficiencyEstimator {
    active_power_integrator: Integrator<Hours, WattHours>,
    residual_energy_integrator: Integrator<Hours, WattHours>,
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
        residual_energy_sample: Integrator<Hours, WattHours>,
        active_power_lhs: Watts,
        active_power_rhs: Watts,
    ) {
        self.active_power_integrator += Integrator::trapezoid(
            residual_energy_sample.weight,
            active_power_lhs,
            active_power_rhs,
        );
        self.residual_energy_integrator += residual_energy_sample;
    }

    pub fn sub_assign_residual_energy(&mut self, power: Watts) {
        self.residual_energy_integrator.value -= power * self.residual_energy_integrator.weight;
    }

    /// Estimate efficiency of residual energy change to the active power integral.
    ///
    /// Note that for discharging, this will normally be greater than one.
    pub fn estimate(self) -> f64 {
        self.residual_energy_integrator
            .mean()
            .zip(self.active_power_integrator.mean())
            .map(|(residual_energy, active_energy)| residual_energy / active_energy)
            .filter(|it| it.is_finite())
            .unwrap_or(1.0)
    }
}
