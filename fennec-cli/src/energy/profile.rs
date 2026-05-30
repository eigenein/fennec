use std::f64::consts::TAU;

use chrono::{DateTime, Local, NaiveTime, Timelike};
use musli::{Decode, Encode};

use super::Balance;
use crate::{
    Interval,
    api,
    math::{
        fourier::Harmonic,
        smoothing::{Exponential, HalfLife},
    },
    ops::musli::File,
    prelude::*,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
};

#[must_use]
#[derive(Encode, Decode)]
pub struct Profile {
    /// Timestamp of the last update to the moving exponentials.
    #[musli(Binary, name = 6)]
    #[musli(with = crate::ops::musli::chrono)]
    last_updated_at: DateTime<Local>,

    /// Average EPS active power.
    #[musli(Binary, name = 7)]
    pub eps_active_power: Exponential<Watts>,

    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 8)]
    pub mean_balance: Exponential<Balance<Watts>>,

    /// Energy balance harmonics (c₁ and so on).
    #[musli(Binary, name = 9)]
    pub balance_harmonics: Vec<Exponential<Harmonic<Balance<Watts>>>>,

    /// Battery metrics, updated if and only if when the residual charge changes.
    #[musli(Binary, name = 10)]
    #[musli(default)]
    pub battery_metrics: Option<api::battery::Metrics>,

    #[musli(Binary, name = 11)]
    #[musli(default)]
    pub battery_efficiency: crate::battery::Efficiency,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            last_updated_at: Local::now(),
            mean_balance: Exponential(Balance::ZERO),
            eps_active_power: Exponential(Watts::ZERO),

            // TODO: make number of harmonics configurable?
            balance_harmonics: vec![Exponential(Harmonic::ZERO); 8],

            battery_metrics: None,
            battery_efficiency: crate::battery::Efficiency::default(),
        }
    }
}

impl File for Profile {
    const PATH: &str = "energy-profile.musli";
}

impl Profile {
    #[instrument(skip_all)]
    pub fn update_battery_metrics(
        &mut self,
        current_metrics: api::battery::Metrics,
        half_life: HalfLife,
    ) {
        let Some(last_metrics) = &self.battery_metrics else {
            // First initialization:
            self.battery_metrics = Some(current_metrics);
            return;
        };

        let residual_energy_change =
            WattHours::from(current_metrics.residual_energy() - last_metrics.residual_energy());
        if residual_energy_change == Zero::ZERO {
            // No change in the residual energy: do not update the parameters and keep accumulating.
            return;
        }

        let grid_flow = current_metrics.total_grid_flow - last_metrics.total_grid_flow;
        let elapsed = current_metrics.timestamp - last_metrics.timestamp;
        let smoothing_factor = half_life.smoothing_factor(elapsed);
        info!(?residual_energy_change, ?grid_flow.import, ?grid_flow.export, %elapsed, ?smoothing_factor, "updating battery efficiency");
        let elapsed = Hours::from(elapsed);
        let parasitic_loss = self.battery_efficiency.parasitic_load.0 * elapsed;

        match (grid_flow.import == Zero::ZERO, grid_flow.export == Zero::ZERO) {
            (true, true) => {
                let parasitic_load = -residual_energy_change / elapsed;
                self.battery_efficiency.parasitic_load.update(parasitic_load, smoothing_factor);
                info!(?parasitic_load, "idling");
            }
            (true, false) => {
                let efficiency =
                    (WattHours::from(grid_flow.export) + parasitic_loss) / -residual_energy_change;
                self.battery_efficiency.discharging.update(efficiency, smoothing_factor);
                info!(?efficiency, "discharging");
            }
            (false, true) => {
                let efficiency =
                    residual_energy_change / (WattHours::from(grid_flow.import) - parasitic_loss);
                self.battery_efficiency.charging.update(efficiency, smoothing_factor);
                info!(?efficiency, "charging");
            }
            (false, false) => {
                // Mixed regime is not good enough for updating the parameters.
            }
        }

        self.battery_metrics = Some(current_metrics);
    }

    pub fn update_energy_balance(
        &mut self,
        balance: Balance<Watts>,
        eps_active_power: Watts,
        at: DateTime<Local>,
        half_life: HalfLife,
    ) {
        let smoothing_factor = {
            let elapsed = at - std::mem::replace(&mut self.last_updated_at, at);
            half_life.smoothing_factor(elapsed)
        };

        self.eps_active_power.update(eps_active_power, smoothing_factor);

        // Deviation is calculated before the mean update eats the signal:
        let deviation = balance - self.mean_balance.0;
        self.mean_balance.update(balance, smoothing_factor);

        // Capture daily periodicity, hence one full day is τ radians:
        let base_phase = f64::from(at.time().num_seconds_from_midnight()) / 86400.0 * TAU;
        for (k, harmonic) in (1..).zip(self.balance_harmonics.iter_mut()) {
            harmonic.update(Harmonic::project(deviation, base_phase, k), smoothing_factor);
        }
    }

    /// Calculate the balance deviation from the average at concrete moment in time.
    pub fn deviation_at(&self, naive_time: NaiveTime) -> Balance<Watts> {
        let day_phase = f64::from(naive_time.num_seconds_from_midnight()) / 86400.0 * TAU;
        (1..)
            .zip(self.balance_harmonics.iter())
            .map(|(k, harmonic)| {
                let phase = day_phase * f64::from(k);
                harmonic.0.cosine * phase.cos() + harmonic.0.sine * phase.sin()
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }

    pub fn mean_balance_over(&self, interval: Interval) -> Balance<Watts> {
        let balance = self.mean_balance.0 + self.mean_deviation_over(interval);
        Balance { grid: balance.grid.normalized(), battery: balance.battery.normalized() }
    }

    /// Calculate the mean deviation of the balance over the interval.
    fn mean_deviation_over(&self, interval: Interval) -> Balance<Watts> {
        assert!(interval.start() < interval.end());

        // The harmonics are periodic over 24 hours, so we only care about the naive time:
        let start_time = f64::from(interval.start().time().num_seconds_from_midnight()) / 86400.0;
        let end_time = f64::from(interval.end().time().num_seconds_from_midnight()) / 86400.0;
        let n_days = interval.duration().days();

        (1..)
            .zip(self.balance_harmonics.iter())
            .map(|(k, harmonic)| {
                let angular_frequency = TAU * f64::from(k);
                let cosine_mean = ((angular_frequency * end_time).sin()
                    - (angular_frequency * start_time).sin())
                    / angular_frequency
                    / n_days;
                let sine_mean = ((angular_frequency * start_time).cos()
                    - (angular_frequency * end_time).cos())
                    / angular_frequency
                    / n_days;
                harmonic.0.cosine * cosine_mean + harmonic.0.sine * sine_mean
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }
}
