use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;

use crate::{
    api::fox_cloud,
    battery::WorkingMode,
    cli::{
        battery::BatteryArgs,
        connection::{ConnectionArgs, Connections},
    },
    cron::CronSchedule,
    db::power,
    energy,
    fmt::tables::build_steps_table,
    ops::Interval,
    prelude::*,
    quantity::{Quantum, energy::WattHours, price::KilowattHourPrice},
    solution::Solver,
    state::HunterState,
    web,
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
        default_value = "idle,harness,charge,compensate",
    )]
    working_modes: Vec<WorkingMode>,

    #[clap(long = "quantum-watthours", env = "QUANTUM_WATTHOURS", default_value = "1")]
    quantum: WattHours,

    #[clap(flatten)]
    connections: ConnectionArgs,

    #[clap(flatten)]
    battery: BatteryArgs,
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
                .build(),
        ))
    }
}

#[derive(Parser)]
pub struct HuntOnceArgs {
    #[clap(flatten)]
    shared: HuntSharedArgs,
}

impl HuntOnceArgs {
    pub async fn run(self) -> Result {
        let (connections, hunter) = self.shared.hunter().await?;
        let result = hunter.run_once().await;
        connections.db.shutdown().await;
        drop(result?);
        Ok(())
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
}

impl Hunter {
    const BACKOFF: ExponentialBuilder =
        ExponentialBuilder::new().with_min_delay(Duration::from_secs(10));

    pub async fn run_forever(
        self,
        schedule: CronSchedule,
        component: web::application::Component<HunterState>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            component.update(self.run_once().await.context("the hunter iteration has failed")?);
        }
    }

    #[instrument(skip_all)]
    pub async fn run_once(&self) -> Result<HunterState> {
        let now = Local::now().with_nanosecond(0).unwrap();
        let energy_prices =
            (|| self.get_prices(now)).retry(Self::BACKOFF).notify(log_error).await?;

        let battery_state = (async || self.connections.battery.lock().await.read_state().await)
            .retry(Self::BACKOFF)
            .notify(log_error)
            .await
            .context("failed to read the battery state")?;
        info!(
            charge = ?battery_state.charge,
            health = ?battery_state.health,
            actual_capacity = ?battery_state.actual_capacity(),
            min_system_charge = ?battery_state.min_system_charge,
            charge_range = ?battery_state.charge_range,
            "battery state",
        );

        let energy_profile = {
            let power_logs = self.connections.db.measurements::<power::Measurement>().await?;
            energy::Profile::try_estimate(
                self.battery_args.power_limits,
                self.energy_provider.time_step(),
                power_logs,
            )
            .await?
        };

        let initial_energy_level = self.quantum.index(battery_state.residual_energy()).unwrap();
        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .balance_profile(&energy_profile)
            .working_modes(self.working_modes)
            .min_residual_energy(battery_state.min_residual_energy())
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                battery_state.max_residual_energy().max(battery_state.residual_energy()),
            )
            .battery_efficiency(energy_profile.battery_efficiency)
            .purchase_fee(self.energy_provider.purchase_fee())
            .now(now)
            .quantum(self.quantum)
            .max_battery_flow(
                self.battery_args.power_limits.max_effective_flow(energy_profile.average_eps_power),
            )
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let base_loss = solver.base_loss();
        let (metrics, steps) = solver.solve().backtrack(initial_energy_level)?;
        println!("{}", build_steps_table(&steps));
        info!(
            profit = ?(base_loss - metrics.losses.total()),
            ?base_loss,
            grid_loss = ?metrics.losses.grid,
            battery.loss = ?metrics.losses.battery,
            battery.charge = ?metrics.internal_battery_flow.import,
            battery.discharge = ?metrics.internal_battery_flow.export,
            "solution summary",
        );

        let schedule = steps.iter().map(|step| (step.interval, step.working_mode)).collect_vec();
        let groups = fox_cloud::Groups::from_schedule(&schedule, self.battery_args.power_limits);
        println!("{}", &groups);

        if let Some(fox_cloud) = &self.connections.fox_cloud {
            (|| fox_cloud.set_schedule(groups.as_ref()))
                .retry(Self::BACKOFF)
                .notify(log_error)
                .await?;
        } else {
            warn!("not pushing the schedule to Fox Cloud, just scouting");
        }

        Ok(HunterState { steps, base_loss, metrics, energy_profile })
    }

    /// Fetch energy prices for up to 2 days.
    #[instrument(skip_all, fields(now = ?now))]
    async fn get_prices(&self, now: DateTime<Local>) -> Result<Vec<(Interval, KilowattHourPrice)>> {
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
