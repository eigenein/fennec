use std::{sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use enumset::EnumSet;
use itertools::Itertools;
use tokio::sync::RwLock;

use crate::{
    Interval,
    Schedule,
    api,
    battery::WorkingMode,
    cli::{battery::BatteryArgs, connection::Connections},
    cron::CronSchedule,
    energy,
    energy::Flow,
    prelude::*,
    quantity::{energy::WattHours, price::KilowattHourPrice},
    solution,
    solution::{Solver, Step},
};

#[must_use]
#[derive(Builder)]
pub struct Runner {
    connections: Connections,
    battery_args: BatteryArgs,
    working_modes: EnumSet<WorkingMode>,
    energy_provider: energy::Provider,
    n_balance_harmonics: usize,
    scout: bool,
}

impl Runner {
    const BACKOFF: ExponentialBuilder =
        ExponentialBuilder::new().with_min_delay(Duration::from_secs(10));

    pub async fn run_forever(self, schedule: CronSchedule, state: Arc<RwLock<State>>) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *state.write().await =
                self.run_once().await.context("the hunter iteration has failed")?;
        }
    }

    #[instrument(skip_all)]
    pub async fn run_once(&self) -> Result<State> {
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

        // FIXME: reading it on every iteration is meh.
        let energy_profile = energy::Profile::read_from_file(self.n_balance_harmonics).await?;

        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .energy_profile(&energy_profile)
            .working_modes(self.working_modes)
            .allowed_residual_energy(
                (battery_state.actual_capacity() * self.battery_args.charge_limits.min)
                    ..=(battery_state.actual_capacity() * self.battery_args.charge_limits.max),
            )
            .battery_efficiency(energy_profile.battery_efficiency_estimator.as_efficiency())
            .now(now)
            .max_battery_flow(
                self.battery_args
                    .power_limits
                    .max_effective_flow(energy_profile.eps_active_power.0),
            )
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let solutions = solver.solve();
        let initial_energy_level = WattHours::from(battery_state.residual_energy()).into();
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
            api::battery::schedule::build(
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

        Ok(State { steps, metrics })
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

#[must_use]
pub struct State {
    pub steps: Vec<((Interval, Flow<KilowattHourPrice>), Step)>,
    pub metrics: solution::Metrics,
}
