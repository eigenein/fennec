use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use bon::Builder;
use chrono::{DateTime, Days, Local, Timelike};
use clap::Parser;
use enumset::EnumSet;
use itertools::Itertools;
use tokio::{spawn, try_join};

use crate::{
    api::fox_cloud,
    battery::WorkingMode,
    cli::{
        battery::BatteryArgs,
        connection::{ConnectionArgs, Connections},
        log::Logger,
    },
    cron::CronSchedule,
    db::power,
    energy,
    fmt::tables::build_steps_table,
    ops::Interval,
    prelude::*,
    quantity::{Quantum, energy::WattHours, price::KilowattHourPrice},
    solution::Solver,
    state::SolverState,
    web,
    web::state::{ApplicationState, SystemState},
};

#[derive(Parser)]
pub struct HuntArgs {
    #[clap(flatten)]
    shared: HuntSharedArgs,

    #[clap(long = "logger-cron", env = "LOGGER_CRON", default_value = "*/5 * * * * *")]
    logger_cron: CronSchedule,

    #[clap(long = "optimizer-cron", env = "OPTIMIZER_CRON", default_value = "0 */15 * * * *")]
    optimizer_cron: CronSchedule,

    #[clap(long = "power-log-ttl", env = "POWER_LOG_TTL", default_value = "14days")]
    power_log_ttl: humantime::Duration,

    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "0.0.0.0")]
    bind_address: IpAddr,

    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    bind_port: u16,
}

impl HuntArgs {
    pub async fn run(self) -> Result {
        let (connections, hunter) = self.shared.hunter().await?;
        connections.db.set_expiration_time::<power::Measurement>(self.power_log_ttl.into()).await?;

        let (logger_result, hunter_result, web_result) = {
            let application_state = ApplicationState::default();
            let logger = Logger::builder()
                .connections(connections.clone())
                .system_state(application_state.logger.clone())
                .build();
            try_join!(
                spawn(logger.run(self.logger_cron)),
                spawn(hunter.run(self.optimizer_cron, application_state.solver.clone())),
                spawn(web::serve(self.bind_address, self.bind_port, application_state)),
            )?
        };

        connections.db.shutdown().await;
        logger_result.and(hunter_result).and(web_result)
    }
}

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
    async fn run(
        self,
        schedule: CronSchedule,
        system_state: Arc<Mutex<SystemState<SolverState>>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *system_state.lock().unwrap() = match self.run_once().await {
                Ok(solver_state) => SystemState::ok(solver_state),
                Err(error) => {
                    error!("hunter iteration failed: {error:#}");
                    SystemState::Err(error)
                }
            };
        }
    }

    #[instrument(skip_all)]
    async fn run_once(&self) -> Result<SolverState> {
        let now = Local::now().with_nanosecond(0).unwrap();
        let energy_prices = self.get_prices(now).await?;

        let battery_state = self.connections.battery.lock().await.read_state().await?;
        println!("{battery_state}");

        let balance_profile = {
            let power_logs = self.connections.db.measurements::<power::Measurement>().await?;
            energy::BalanceProfile::try_estimate(
                self.battery_args.power_limits,
                self.energy_provider.time_step(),
                power_logs,
            )
            .await?
        };

        let initial_energy_level = self.quantum.index(battery_state.residual_energy()).unwrap();
        let solver = Solver::builder()
            .energy_prices(&energy_prices)
            .balance_profile(&balance_profile)
            .working_modes(self.working_modes)
            .min_residual_energy(battery_state.min_residual_energy())
            .max_residual_energy(
                // Current residual may be higher than the maximum SoC setting:
                battery_state.max_residual_energy().max(battery_state.residual_energy()),
            )
            .battery_efficiency(self.battery_args.efficiency)
            .purchase_fee(self.energy_provider.purchase_fee())
            .now(now)
            .quantum(self.quantum)
            .max_battery_flow(
                self.battery_args
                    .power_limits
                    .max_effective_flow(balance_profile.average_eps_power),
            )
            .battery_degradation_cost(self.battery_args.degradation_cost)
            .build();
        let base_loss = solver.base_loss();
        let (summary, steps) = solver.solve().backtrack(initial_energy_level)?;
        println!("{}", build_steps_table(&steps));
        println!("{}", summary.into_table(base_loss));

        let schedule = steps.iter().map(|step| (step.interval, step.working_mode)).collect_vec();
        let groups = fox_cloud::Groups::from_schedule(&schedule, self.battery_args.power_limits);
        println!("{}", &groups);

        if let Some(fox_cloud) = &self.connections.fox_cloud {
            fox_cloud.set_schedule(groups.as_ref()).await?;
        } else {
            warn!("not pushing the schedule to Fox Cloud, just scouting");
        }

        Ok(SolverState { steps, actual_capacity: battery_state.actual_capacity() })
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
