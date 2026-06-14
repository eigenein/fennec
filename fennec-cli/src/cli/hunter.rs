use std::{sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use tokio::sync::RwLock;

use crate::{
    Schedule,
    api,
    cli::{BatteryArgs, Connections},
    cron::CronSchedule,
    energy,
    energy::Flow,
    prelude::*,
    quantity::{
        energy::{EnergyLevel, WattHours},
        price::KilowattHourPrice,
    },
    solution,
    solution::{Solver, Step},
};

#[must_use]
#[derive(Builder)]
pub struct Runner {
    connections: Connections,
    battery_args: BatteryArgs,
    energy_provider: energy::Provider,
    n_balance_harmonics: usize,
    dry_run: bool,
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

        let battery_state = (async || self.connections.battery.read_metrics().await)
            .retry(Self::BACKOFF)
            .notify(log_retried_error)
            .await
            .context("failed to read the battery state")?;
        info!(
            charge = ?battery_state.tracked.charge,
            health = ?battery_state.tracked.health,
            actual_capacity = ?battery_state.tracked.actual_capacity(),
            "battery state",
        );

        // FIXME: do not re-read it when the hunter and logger would be combined.
        let energy_profile = energy::Profile::read_from_file(self.n_balance_harmonics).await?;

        let min_energy_level = EnergyLevel::from(battery_state.min_residual_charge());
        let max_energy_level = EnergyLevel::from(battery_state.max_residual_charge());
        let solver = Solver::builder()
            .working_modes(self.battery_args.working_modes.iter().copied().collect())
            .allowed_energy_levels(min_energy_level..=max_energy_level)
            .battery_efficiency(energy_profile.battery_efficiency)
            .battery_capacity(battery_state.tracked.actual_capacity())
            .max_battery_flow(
                self.battery_args
                    .power_limits
                    .max_effective_flow(energy_profile.eps_active_power.0),
            )
            .energy_profile(energy_profile)
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let initial_energy_level = WattHours::from(battery_state.tracked.residual_energy()).into();
        let (metrics, steps) = solver.solve(energy_prices).space.backtrack(initial_energy_level)?;
        info!(
            grid_loss = ?metrics.losses.grid,
            battery.loss = ?metrics.losses.battery,
            battery.charge = ?metrics.internal_battery_flow.import,
            battery.discharge = ?metrics.internal_battery_flow.export,
            "solution summary",
        );

        let entries = {
            let schedule = steps.iter().map(|slot| (slot.interval, slot.value.1.working_mode));
            api::battery::schedule::build(
                schedule,
                battery_state.untracked.allowed_charge,
                self.battery_args.power_limits,
            )
        };

        if self.dry_run {
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
        ensure!(prices.len() != 0, "received empty price schedule");

        let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
        prices.extend(self.energy_provider.get_prices(tomorrow).await?)?;

        info!(len = prices.len(), "fetched energy prices");
        prices.advance_to(now);
        Ok(prices)
    }
}

#[must_use]
pub struct State {
    pub steps: Schedule<(Flow<KilowattHourPrice>, Step)>,
    pub metrics: solution::Metrics,
}
