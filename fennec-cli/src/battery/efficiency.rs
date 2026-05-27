use std::time::Instant;

use anyhow::{Context, Error};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use tracing::info;

use crate::{
    db::power,
    math::Integrator,
    prelude::instrument,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
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

    #[instrument(skip_all)]
    pub async fn try_estimate<T>(mut logs: T) -> crate::prelude::Result<Self>
    where
        T: TryStream<Ok = power::Measurement, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");
        let start_time = Instant::now();

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;

        let mut parasitic_power_integrator = Integrator::new();
        let mut charging_efficiency_estimator = EfficiencyEstimator::new();
        let mut discharging_efficiency_estimator = EfficiencyEstimator::new();

        while let Some(current) = logs.try_next().await? {
            let duration = Hours::from(current.timestamp - previous.timestamp);

            let residual_energy_sample =
                // The value sign here matches the active power sign, so charging is negative:
                Integrator { weight: duration, value: previous.battery.residual_energy - current.battery.residual_energy };

            if previous.battery.active_power == Watts::ZERO
                && current.battery.active_power == Watts::ZERO
            {
                parasitic_power_integrator += residual_energy_sample;
            } else if previous.battery.active_power > Watts::ZERO
                && current.battery.active_power > Watts::ZERO
            {
                discharging_efficiency_estimator.push(
                    residual_energy_sample,
                    previous.battery.active_power,
                    current.battery.active_power,
                );
            } else if previous.battery.active_power < Watts::ZERO
                && current.battery.active_power < Watts::ZERO
            {
                charging_efficiency_estimator.push(
                    residual_energy_sample,
                    previous.battery.active_power,
                    current.battery.active_power,
                );
            }

            previous = current;
        }

        let parasitic_load = parasitic_power_integrator.mean().unwrap_or(Watts::ZERO);
        charging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        discharging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        let this = Self {
            charging: charging_efficiency_estimator.estimate().clamp(0.5, 1.5),
            discharging: (1.0 / discharging_efficiency_estimator.estimate()).clamp(0.5, 1.5),
            parasitic_load,
        };

        info!(
            this.charging,
            this.discharging,
            battery_round_trip_efficiency = this.round_trip(),
            ?parasitic_load,
            elapsed = ?start_time.elapsed(),
            "done",
        );

        Ok(this)
    }
}

#[must_use]
#[derive(Copy, Clone)]
struct EfficiencyEstimator {
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
