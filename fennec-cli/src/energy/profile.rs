use std::{fmt::Debug, path::Path, time::Instant};

use chrono::{DateTime, Local, NaiveTime, TimeDelta, Timelike};
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
            BucketIntegrator::new(bucket_time_step.index(max_naive_time).unwrap())
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
                    let previous_bucket =
                        bucket_time_step.index(previous_timestamp.time()).unwrap();
                    let next_bucket = bucket_time_step.index(current_timestamp.time()).unwrap();
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
        self.average_balance[self.time_step.index(time).unwrap()]
    }
}

/// TODO: merge into [`State`], pass `decay` from outside.
#[must_use]
pub struct Manager {
    decay: HalfLife,
    state: State,
}

impl Manager {
    pub const PATH: &str = "energy-profile.musli";

    pub async fn read_or_default(decay: HalfLife) -> Result<Self> {
        let path = Path::new(Self::PATH);
        let state = if path.exists() {
            info!("reading energy profile…");
            State::read_from(path).await?
        } else {
            info!("creating new energy profile");
            State::new()
        };
        Ok(Self { decay, state })
    }

    pub fn update(&mut self, balance: Balance<Watts>, at: DateTime<Local>) -> &Self {
        self.state.average.update(balance, at, self.decay);

        let deviation = balance - *self.state.average.get();
        self.state.deviations[State::index(at.time())].update(deviation, at, self.decay);

        self
    }

    pub async fn write(&self) -> Result {
        self.state.write_to(Path::new(Self::PATH)).await
    }
}

/// Persistent state of the energy profile.
///
/// TODO: rename into `Profile`, when the above is gone.
#[must_use]
#[derive(Encode, Decode)]
pub struct State {
    /// Global average energy balance.
    #[musli(Binary, name = 1)]
    average: Clocked<Balance<Watts>>,

    /// Energy balance deviation from the global average per [`Self::N_MINUTES_PER_SLOT`] minutes.
    #[musli(Binary, name = 2)]
    deviations: [Clocked<Balance<Watts>>; Self::N_SLOTS],
}

impl State {
    const N_MINUTES_PER_SLOT: usize = 5;
    const N_SLOTS: usize = 1440 / Self::N_MINUTES_PER_SLOT;

    fn index(for_: NaiveTime) -> usize {
        (for_.hour() * 60 + for_.minute()) as usize / Self::N_MINUTES_PER_SLOT
    }

    fn new() -> Self {
        let now = Local::now();
        let deviations = std::array::from_fn(|_| Clocked::new(Balance::ZERO, now));
        Self { average: Clocked::new(Balance::ZERO, now), deviations }
    }

    #[instrument(skip_all, fields(path = ?path))]
    pub async fn read_from(path: impl AsRef<Path> + Debug) -> Result<Self> {
        let bytes = tokio::fs::read(path).await.context("failed to read the energy profile")?;
        wire::decode(bytes.as_slice()).context("failed to decode the energy profile")
    }

    pub const fn get_average(&self) -> Balance<Watts> {
        *self.average.get()
    }

    #[instrument(skip_all, fields(path = ?path))]
    async fn write_to(&self, path: impl AsRef<Path> + Debug) -> Result {
        let bytes = wire::to_vec(&self).context("failed to encode the energy profile")?;
        tokio::fs::write(path, bytes.as_slice())
            .await
            .context("failed to write the energy profile")?;
        Ok(())
    }
}
