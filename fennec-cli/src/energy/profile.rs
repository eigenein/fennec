use std::f64::consts::TAU;

use chrono::{DateTime, Local, NaiveTime, Timelike};
use musli::{Decode, Encode};

use super::Balance;
use crate::{
    Interval,
    battery,
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
    eps_active_power: Exponential<Watts>,

    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 8)]
    mean_balance: Exponential<Balance<Watts>>,

    /// Energy balance harmonics (c₁ and so on).
    #[musli(Binary, name = 9)]
    balance_harmonics: Vec<Exponential<Harmonic<Balance<Watts>>>>,

    /// Battery metrics, updated if and only if when the residual charge changes.
    #[musli(Binary, name = 10)]
    #[musli(default)]
    battery_metrics: Option<battery::Metrics>,

    #[musli(Binary, name = 11)]
    #[musli(default)]
    battery_efficiency: battery::Efficiency,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            last_updated_at: Local::now(),
            mean_balance: Exponential::new(Balance::ZERO),
            eps_active_power: Exponential::new(Watts::ZERO),

            // TODO: make number of harmonics configurable?
            balance_harmonics: vec![Exponential::new(Harmonic::ZERO); 8],

            battery_metrics: None,
            battery_efficiency: battery::Efficiency::default(),
        }
    }
}

impl File for Profile {
    const PATH: &str = "energy-profile.musli";
}

impl Profile {
    pub const fn eps_active_power(&self) -> Watts {
        *self.eps_active_power.value()
    }

    pub const fn mean_balance(&self) -> Balance<Watts> {
        *self.mean_balance.value()
    }

    pub const fn balance_harmonics(&self) -> &[Exponential<Harmonic<Balance<Watts>>>] {
        self.balance_harmonics.as_slice()
    }

    pub const fn battery_metrics(&self) -> Option<&battery::Metrics> {
        self.battery_metrics.as_ref()
    }

    pub const fn battery_charging_efficiency(&self) -> f64 {
        *self.battery_efficiency.charging.value()
    }

    pub const fn battery_discharging_efficiency(&self) -> f64 {
        *self.battery_efficiency.discharging.value()
    }

    pub const fn battery_round_trip_efficiency(&self) -> f64 {
        self.battery_charging_efficiency() * self.battery_discharging_efficiency()
    }

    pub const fn battery_parasitic_load(&self) -> Watts {
        *self.battery_efficiency.parasitic_load.value()
    }

    #[instrument(skip_all)]
    pub fn update_battery_metrics(
        &mut self,
        current_battery_metrics: battery::Metrics,
        half_life: HalfLife,
    ) {
        if let Some(last_battery_metrics) = &self.battery_metrics {
            let residual_energy_change = WattHours::from(
                current_battery_metrics.residual_energy() - last_battery_metrics.residual_energy(),
            );
            if residual_energy_change == Zero::ZERO {
                return;
            }

            let grid_flow =
                current_battery_metrics.total_grid_flow - last_battery_metrics.total_grid_flow;
            let elapsed = current_battery_metrics.timestamp - last_battery_metrics.timestamp;
            let smoothing_factor = half_life.smoothing_factor(elapsed);
            info!(%elapsed, ?smoothing_factor, "updating battery efficiency");
            let elapsed = Hours::from(elapsed);

            if grid_flow.import == Zero::ZERO {
                if grid_flow.export == Zero::ZERO {
                    let parasitic_load = -residual_energy_change / elapsed;
                    self.battery_efficiency.parasitic_load.update(parasitic_load, smoothing_factor);
                    info!(?parasitic_load);
                } else {
                    let parasitic_energy =
                        *self.battery_efficiency.parasitic_load.value() * elapsed;
                    let efficiency = (WattHours::from(grid_flow.export) + parasitic_energy)
                        / -residual_energy_change;
                    self.battery_efficiency.discharging.update(efficiency, smoothing_factor);
                    info!(?efficiency, "discharging");
                }
            } else if grid_flow.export == Zero::ZERO {
                let parasitic_energy = *self.battery_efficiency.parasitic_load.value() * elapsed;
                let efficiency =
                    residual_energy_change / (WattHours::from(grid_flow.import) - parasitic_energy);
                self.battery_efficiency.charging.update(efficiency, smoothing_factor);
                info!(?efficiency, "charging");
            }
        }
        self.battery_metrics = Some(current_battery_metrics);
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
        let deviation = balance - *self.mean_balance.value();
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
                harmonic.value().cosine * phase.cos() + harmonic.value().sine * phase.sin()
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }

    pub fn mean_balance_over(&self, interval: Interval) -> Balance<Watts> {
        let balance = *self.mean_balance.value() + self.mean_deviation_over(interval);
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
                let harmonic = harmonic.value();
                let cosine_mean = ((angular_frequency * end_time).sin()
                    - (angular_frequency * start_time).sin())
                    / angular_frequency
                    / n_days;
                let sine_mean = ((angular_frequency * start_time).cos()
                    - (angular_frequency * end_time).cos())
                    / angular_frequency
                    / n_days;
                harmonic.cosine * cosine_mean + harmonic.sine * sine_mean
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }
}
