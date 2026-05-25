use std::{f64::consts::TAU, ops::Mul, path::Path, time::Instant};

use chrono::{DateTime, Local, NaiveTime, TimeDelta, Timelike};
use derive_more::{AddAssign, Sub};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use musli::{Decode, Encode, wire};

use super::Balance;
use crate::{
    battery,
    battery::EfficiencyEstimator,
    cli::battery::PowerLimits,
    db::power,
    ops::{
        BucketIntegrator,
        BucketMean,
        Integrator,
        Interval,
        smoothing::{Clocked, HalfLife},
    },
    prelude::*,
    quantity::{Quantum, Zero, power::Watts, time::Hours},
};

/// TODO: reduce to battery profile and move under `battery`.
#[must_use]
pub struct Profile {
    pub average_eps_power: Watts,
    pub battery_efficiency: battery::Efficiency,

    time_step: TimeDelta,
    average_balance: BucketMean<Balance<Watts>>,
}

impl Profile {
    #[instrument(skip_all)]
    pub async fn try_estimate<T>(
        battery_power_limits: PowerLimits,
        bucket_time_step: TimeDelta,
        mut logs: T,
    ) -> Result<Self>
    where
        T: TryStream<Ok = power::Measurement, Error = Error> + Unpin,
    {
        info!("crunching consumption logs…");
        let start_time = Instant::now();

        let mut previous = logs.try_next().await?.context("empty consumption logs")?;

        let mut balance_integrator = {
            let max_naive_time =
                NaiveTime::from_num_seconds_from_midnight_opt(86399, 999_999_999).unwrap();
            BucketIntegrator::new(bucket_time_step.index(max_naive_time))
        };
        let mut eps_power_integrator = Integrator::new();
        let mut parasitic_power_integrator = Integrator::new();
        let mut charging_efficiency_estimator = EfficiencyEstimator::new();
        let mut discharging_efficiency_estimator = EfficiencyEstimator::new();

        while let Some(current) = logs.try_next().await? {
            let duration = Hours::from(current.timestamp - previous.timestamp);

            {
                let sample = Integrator::trapezoid(
                    duration,
                    Balance::new(battery_power_limits, previous.net_deficit),
                    Balance::new(battery_power_limits, current.net_deficit),
                );
                balance_integrator.total += sample;

                let previous_timestamp = previous.timestamp.with_timezone(&Local);
                let current_timestamp = current.timestamp.with_timezone(&Local);

                if previous_timestamp.date_naive() == current_timestamp.date_naive() {
                    let previous_bucket = bucket_time_step.index(previous_timestamp.time());
                    let next_bucket = bucket_time_step.index(current_timestamp.time());
                    if next_bucket == previous_bucket {
                        balance_integrator.buckets[next_bucket] += sample;
                    }
                }
            }

            eps_power_integrator += Integrator::trapezoid(
                duration,
                previous.battery.eps_active_power,
                current.battery.eps_active_power,
            );

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

        let average_eps_power = eps_power_integrator.mean().unwrap_or(Watts::ZERO);

        let parasitic_load = parasitic_power_integrator.mean().unwrap_or(Watts::ZERO);
        charging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        discharging_efficiency_estimator.sub_assign_residual_energy(parasitic_load);
        let battery_efficiency = battery::Efficiency {
            charging: charging_efficiency_estimator.estimate().clamp(0.5, 1.5),
            discharging: (1.0 / discharging_efficiency_estimator.estimate()).clamp(0.5, 1.5),
            parasitic_load,
        };

        info!(
            battery_efficiency.charging,
            battery_efficiency.discharging,
            battery_round_trip_efficiency = battery_efficiency.round_trip(),
            ?average_eps_power,
            ?parasitic_load,
            elapsed = ?start_time.elapsed(),
            "done",
        );

        Ok(Self {
            time_step: bucket_time_step,
            average_balance: balance_integrator.try_into()?, // FIXME: make infallible.
            average_eps_power,
            battery_efficiency,
        })
    }

    pub fn average_balance_on(&self, time: NaiveTime) -> Balance<Watts> {
        self.average_balance[self.time_step.index(time)]
    }
}

/// TODO: rename into `Profile`, when the above is gone.
#[must_use]
#[derive(Encode, Decode)]
pub struct New {
    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 1)]
    mean_balance: Clocked<Balance<Watts>>,

    /// Average EPS active power.
    #[musli(Binary, name = 3)]
    eps_active_power: Clocked<Watts>,

    #[musli(Binary, name = 4, default = Self::default_harmonics)]
    harmonics: Vec<Clocked<Harmonic>>,
}

impl Default for New {
    fn default() -> Self {
        let now = Local::now();
        Self {
            mean_balance: Clocked::new(Balance::ZERO, now),
            eps_active_power: Clocked::new(Watts::ZERO, now),

            // TODO: make number of harmonics configurable?
            harmonics: vec![Clocked::new(Harmonic::ZERO, now); 8],
        }
    }
}

impl New {
    const PATH: &str = "energy-profile.musli";

    pub async fn read_or_default() -> Result<Self> {
        let path = Path::new(Self::PATH);
        if path.exists() {
            info!(?path, "reading energy profile…");
            Self::read().await
        } else {
            info!("creating new energy profile");
            Ok(Self::default())
        }
    }

    pub async fn read() -> Result<Self> {
        let bytes =
            tokio::fs::read(Self::PATH).await.context("failed to read the energy profile")?;
        wire::decode(bytes.as_slice()).context("failed to decode the energy profile")
    }

    /// TODO: write to temporary file and rename for atomicity.
    pub async fn write(&self) -> Result {
        let bytes = wire::to_vec(&self).context("failed to encode the energy profile")?;
        tokio::fs::write(Self::PATH, bytes.as_slice())
            .await
            .context("failed to write the energy profile")?;
        Ok(())
    }

    pub const fn eps_active_power(&self) -> Watts {
        *self.eps_active_power.value()
    }

    pub const fn mean_balance(&self) -> Balance<Watts> {
        *self.mean_balance.value()
    }

    pub const fn harmonics(&self) -> &[Clocked<Harmonic>] {
        self.harmonics.as_slice()
    }

    pub fn update(
        &mut self,
        balance: Balance<Watts>,
        eps_active_power: Watts,
        at: DateTime<Local>,
        half_life: HalfLife,
    ) {
        self.eps_active_power.update(eps_active_power, at, half_life);

        // Deviation is calculated before the mean update eats the signal:
        let deviation = balance - *self.mean_balance.value();
        self.mean_balance.update(balance, at, half_life);

        // Capture daily periodicity, hence one full day is τ radians:
        let day_phase = f64::from(at.time().num_seconds_from_midnight()) / 86400.0 * TAU;
        for (k, harmonic) in (1..).zip(self.harmonics.iter_mut()) {
            harmonic.update(Harmonic::project(deviation, day_phase * f64::from(k)), at, half_life);
        }
    }

    pub fn deviation_at(&self, naive_time: NaiveTime) -> Balance<Watts> {
        let day_phase = f64::from(naive_time.num_seconds_from_midnight()) / 86400.0 * TAU;
        (1..)
            .zip(self.harmonics.iter())
            .map(|(k, harmonic)| {
                let phase = day_phase * f64::from(k);
                harmonic.value().cosine * phase.cos() + harmonic.value().sine * phase.sin()
            })
            .fold(Balance::ZERO, |sum, item| sum + item)
    }

    /// Calculate the mean deviation of the balance over the interval.
    pub fn mean_deviation_over(&self, interval: Interval) -> Balance<Watts> {
        assert_ne!(interval.start(), interval.end());

        let start_time = f64::from(interval.start().time().num_seconds_from_midnight()) / 86400.0;
        let end_time = f64::from(interval.end().time().num_seconds_from_midnight()) / 86400.0;
        let n_days = interval.duration().days();

        (1..)
            .zip(self.harmonics.iter())
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

    fn default_harmonics() -> Vec<Clocked<Harmonic>> {
        vec![Clocked::new(Harmonic::ZERO, Local::now()); 8]
    }
}

/// Single non-constant term of the [decomposition][1].
///
/// [1]: https://en.wikipedia.org/wiki/Fourier_series
#[derive(Clone, AddAssign, Sub, Encode, Decode)]
pub struct Harmonic {
    #[musli(Binary, name = 1)]
    cosine: Balance<Watts>,

    #[musli(Binary, name = 2)]
    sine: Balance<Watts>,
}

impl Zero for Harmonic {
    const ZERO: Self = Self { cosine: Balance::ZERO, sine: Balance::ZERO };
}

impl Mul<f64> for Harmonic {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self { cosine: self.cosine * rhs, sine: self.sine * rhs }
    }
}

impl Harmonic {
    /// Project the signal onto the harmonic.
    pub fn project(signal: Balance<Watts>, phase: f64) -> Self {
        Self { cosine: signal * (2.0 * phase.cos()), sine: signal * (2.0 * phase.sin()) }
    }

    pub const fn cosine(&self) -> Balance<Watts> {
        self.cosine
    }

    pub const fn sine(&self) -> Balance<Watts> {
        self.sine
    }
}
