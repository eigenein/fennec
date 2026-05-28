use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use backon::{ExponentialBuilder, Retryable};
use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use enumset::EnumSet;
use itertools::Itertools;

use crate::{
    Schedule,
    api::modbus::schedule,
    battery,
    battery::WorkingMode,
    cli::{battery::BatteryArgs, connection::Connections, state},
    cron::CronSchedule,
    db::power,
    energy,
    energy::Flow,
    ops::{cache, musli::File},
    prelude::*,
    quantity::{energy::WattHours, price::KilowattHourPrice},
    solution::Solver,
};

#[must_use]
#[derive(Builder)]
pub struct Hunter {
    connections: Connections,
    battery_args: BatteryArgs,
    working_modes: EnumSet<WorkingMode>,
    energy_provider: energy::Provider,
    quantum: WattHours,
    scout: bool,

    #[builder(skip = cache::Ttl::new(Duration::from_hours(1)))]
    battery_efficiency_cache: cache::Ttl<battery::Efficiency>,
}

impl Hunter {
    const BACKOFF: ExponentialBuilder =
        ExponentialBuilder::new().with_min_delay(Duration::from_secs(10));

    pub async fn run_forever(
        mut self,
        schedule: CronSchedule,
        state: Arc<RwLock<state::Hunter>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *state.write().unwrap() =
                self.run_once().await.context("the hunter iteration has failed")?;
        }
    }

    #[instrument(skip_all)]
    pub async fn run_once(&mut self) -> Result<state::Hunter> {
        let now = Local::now().with_nanosecond(0).unwrap();
        let energy_prices =
            (|| self.get_prices(now)).retry(Self::BACKOFF).notify(log_retried_error).await?;

        let battery_state = (async || self.connections.battery.read_state().await)
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
            .context("failed to read the battery state")?;
        info!(
            charge = ?battery_state.charge,
            health = ?battery_state.health,
            actual_capacity = ?battery_state.actual_capacity(),
            "battery state",
        );

        let battery_efficiency = *self
            .battery_efficiency_cache
            .get_or_insert_with(async {
                let power_logs = self.connections.db.measurements::<power::Measurement>().await?;
                battery::Efficiency::try_estimate(power_logs).await
            })
            .await?;
        let energy_profile = energy::Profile::read_or_default().await?;

        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .energy_profile(&energy_profile)
            .working_modes(self.working_modes)
            .min_residual_energy(
                battery_state.actual_capacity() * self.battery_args.charge_limits.min,
            )
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                WattHours::from(battery_state.residual_energy())
                    .max(battery_state.actual_capacity() * self.battery_args.charge_limits.max),
            )
            .battery_efficiency(battery_efficiency)
            .now(now)
            .quantum(self.quantum)
            .max_battery_flow(
                self.battery_args
                    .power_limits
                    .max_effective_flow(energy_profile.eps_active_power()),
            )
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let solutions = solver.solve();
        let initial_energy_level = self.quantum.index(battery_state.residual_energy().into());
        let (metrics, steps) = solutions.backtrack(initial_energy_level)?;
        let steps: Vec<_> = energy_prices.into_iter().zip_eq(steps).collect();
        info!(
            grid_loss = ?metrics.losses.grid,
            battery.loss = ?metrics.losses.battery,
            battery.charge = ?metrics.internal_battery_flow.import,
            battery.discharge = ?metrics.internal_battery_flow.export,
            "solution summary",
        );

        let entries = {
            let schedule = steps.iter().map(|((interval, _), step)| (*interval, step.working_mode));
            schedule::build(
                schedule,
                self.battery_args.charge_limits,
                self.battery_args.power_limits,
            )
        };

        if self.scout {
            warn!("not pushing the schedule to the battery, just scouting");
        } else {
            (async || self.connections.battery.write_schedule(&entries).await)
                .retry(Self::BACKOFF)
                .notify(log_retried_error)
                .await
                .context("failed to push the schedule to the battery")?;
        }

        Ok(state::Hunter { steps, metrics, battery_efficiency })
    }

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(&self, now: DateTime<Local>) -> Result<Schedule<Flow<KilowattHourPrice>>> {
        const ONE_DAY: Days = Days::new(1);

        let today = now.date_naive();
        let mut prices = self.energy_provider.get_prices(today).await?;
        ensure!(!prices.is_empty());

        let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
        prices.extend(self.energy_provider.get_prices(tomorrow).await?)?;

        prices.retain(now);
        info!(len = prices.len(), "fetched energy prices");

        Ok(prices)
    }
}
