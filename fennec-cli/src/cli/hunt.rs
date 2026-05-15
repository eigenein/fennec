use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use backon::{ExponentialBuilder, Retryable};
use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;

use crate::{
    api::modbus::schedule,
    battery::WorkingMode,
    cli::{
        battery::BatteryArgs,
        connection::{ConnectionArgs, Connections},
    },
    cron::CronSchedule,
    db::power,
    energy,
    ops::{Interval, cache},
    prelude::*,
    quantity::{Quantum, energy::WattHours, price::KilowattHourPrice},
    solution::Solver,
    state::HunterState,
};

#[derive(Parser)]
pub struct HuntSharedArgs {
    #[clap(long = "energy-provider", env = "ENERGY_PROVIDER")]
    energy_provider: energy::Provider,

    #[clap(
        long = "working-modes",
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "harness,compensate,charge,self-use",
    )]
    working_modes: Vec<WorkingMode>,

    #[clap(long = "quantum-watthours", env = "QUANTUM_WATTHOURS", default_value = "1")]
    quantum: WattHours,

    #[clap(flatten)]
    connections: ConnectionArgs,

    #[clap(flatten)]
    battery: BatteryArgs,

    /// Do not push schedule to the device, dry run.
    #[clap(long = "scout", env = "SCOUT")]
    scout: bool,
}

impl HuntSharedArgs {
    pub async fn hunter(self) -> Result<(Connections, Hunter)> {
        let connections = self.connections.connect().await?;
        Ok((
            connections.clone(),
            Hunter::builder()
                .connections(connections)
                .working_modes(self.working_modes.iter().copied().collect())
                .energy_provider(self.energy_provider)
                .battery_args(self.battery)
                .quantum(self.quantum)
                .scout(self.scout)
                .build(),
        ))
    }
}

#[must_use]
#[derive(Builder)]
pub struct Hunter {
    connections: Connections,
    battery_args: BatteryArgs,
    working_modes: EnumSet<WorkingMode>,
    energy_provider: energy::Provider,
    quantum: WattHours,
    scout: bool,

    /// TODO: custom builder.
    /// TODO: make configurable.
    #[builder(skip = cache::Ttl::new(Duration::from_hours(1)))]
    energy_profile_cache: cache::Ttl<energy::Profile>,
}

impl Hunter {
    const BACKOFF: ExponentialBuilder =
        ExponentialBuilder::new().with_min_delay(Duration::from_secs(10));

    pub async fn run_forever(
        mut self,
        schedule: CronSchedule,
        state: Arc<RwLock<HunterState>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *state.write().unwrap() =
                self.run_once().await.context("the hunter iteration has failed")?;
        }
    }

    #[instrument(skip_all)]
    pub async fn run_once(&mut self) -> Result<HunterState> {
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

        let energy_profile = self
            .energy_profile_cache
            .get_or_insert_with(async {
                let power_logs = self.connections.db.measurements::<power::Measurement>().await?;
                energy::Profile::try_estimate(
                    self.battery_args.power_limits,
                    self.energy_provider.time_step(),
                    power_logs,
                )
                .await
            })
            .await?;

        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .balance_profile(energy_profile)
            .working_modes(self.working_modes)
            .min_residual_energy(
                battery_state.actual_capacity() * self.battery_args.charge_limits.min,
            )
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                battery_state
                    .residual_energy()
                    .max(battery_state.actual_capacity() * self.battery_args.charge_limits.max),
            )
            .battery_efficiency(energy_profile.battery_efficiency)
            .now(now)
            .quantum(self.quantum)
            .max_battery_flow(
                self.battery_args.power_limits.max_effective_flow(energy_profile.average_eps_power),
            )
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let base_loss = solver.base_loss();
        let solution_space = solver.solve();
        let initial_energy_level = self.quantum.index(battery_state.residual_energy()).unwrap();
        let (metrics, steps) = solution_space.backtrack(initial_energy_level)?;
        let steps: Vec<_> = energy_prices.into_iter().zip_eq(steps).collect();
        info!(
            profit = ?(base_loss - metrics.losses.total()),
            ?base_loss,
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

        Ok(HunterState {
            steps,
            base_loss,
            metrics,
            average_eps_power: energy_profile.average_eps_power,
            battery_efficiency: energy_profile.battery_efficiency,
        })
    }

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(
        &self,
        now: DateTime<Local>,
    ) -> Result<Vec<(Interval, energy::Flow<KilowattHourPrice>)>> {
        const ONE_DAY: Days = Days::new(1);

        let today = now.date_naive();
        let mut prices = self.energy_provider.get_prices(today).await?;
        ensure!(!prices.is_empty());

        let tomorrow = today.checked_add_days(ONE_DAY).unwrap();
        prices.extend(self.energy_provider.get_prices(tomorrow).await?);

        prices.retain(|(interval, _)| interval.end > now);
        info!(len = prices.len(), "fetched energy prices");

        Ok(prices)
    }
}
