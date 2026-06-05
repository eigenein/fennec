use std::{sync::Arc, time::Duration};

use clap::Parser;
use tokio::{spawn, sync::RwLock, try_join};

use crate::{
    battery::WorkingMode,
    cli::{battery::BatteryArgs, connection::ConnectionArgs, hunter, logger, web::BindArgs},
    cron::CronSchedule,
    energy,
    math::smoothing::HalfLife,
    prelude::*,
    web,
};

#[derive(Parser)]
pub struct RunArgs {
    #[clap(flatten)]
    connections: ConnectionArgs,

    #[clap(flatten)]
    bind: BindArgs,

    #[clap(flatten)]
    battery: BatteryArgs,

    #[clap(long, env = "LOGGER_CRON", default_value = "*/5 * * * * *")]
    logger_cron: CronSchedule,

    #[clap(long, env = "OPTIMIZER_CRON", default_value = "0 */15 * * * *")]
    optimizer_cron: CronSchedule,

    #[clap(long, env = "POWER_LOG_TTL", default_value = "14days")]
    power_log_ttl: humantime::Duration,

    #[clap(
        long,
        env = "WORKING_MODES",
        value_delimiter = ',',
        num_args = 1..,
        default_value = "harness,compensate,charge,self-use",
    )]
    working_modes: Vec<WorkingMode>,

    #[clap(long, env = "ENERGY_PROVIDER")]
    energy_provider: energy::Provider,

    /// Half-life for exponential moving average when learning the energy balance:
    /// - after τ: the energy profile is 50% adapted to the new routine;
    /// - after 2τ: 75% adapted;
    /// - after 3τ: 87.5% adapted.
    #[clap(long, env = "ENERGY_BALANCE_HALF_LIFE", default_value = "7d")]
    energy_balance_half_life: humantime::Duration,

    /// Battery parameters are learned with exponential moving average.
    /// This factor multiplied by the battery capacity defines the half-life in the units of energy.
    /// The residual energy change is then used to calculate smoothing at each parameter update.
    #[clap(long, env = "BATTERY_EFFICIENCY_HALF_LIFE_FACTOR", default_value = "10")]
    battery_efficiency_half_life_factor: f64,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    dry_run: bool,

    #[clap(long, env = "N_BALANCE_HARMONICS", default_value = "12")]
    n_balance_harmonics: usize,
}

impl RunArgs {
    pub async fn run(self) -> Result {
        let battery_power_limits = self.battery.power_limits;
        let connections = self.connections.connect()?;

        let logger_runner = logger::Args::builder()
            .connections(connections.clone())
            .battery_power_limits(battery_power_limits)
            .energy_balance_half_life(HalfLife(
                Duration::from(self.energy_balance_half_life).into(),
            ))
            .battery_efficiency_half_life_factor(self.battery_efficiency_half_life_factor)
            .n_balance_harmonics(self.n_balance_harmonics)
            .build()
            .start()
            .await?;
        let hunter_runner = hunter::Runner::builder()
            .connections(connections.clone())
            .working_modes(self.working_modes.iter().copied().collect())
            .energy_provider(self.energy_provider)
            .battery_args(self.battery)
            .n_balance_harmonics(self.n_balance_harmonics)
            .scout(self.dry_run)
            .build();

        let hunter_state = Arc::new(RwLock::new(hunter_runner.run_once().await?));
        let state =
            web::State { hunter: hunter_state.clone(), logger_runner: logger_runner.clone() };
        try_join!(
            async { spawn(logger_runner.run_forever(self.logger_cron)).await? },
            async { spawn(hunter_runner.run_forever(self.optimizer_cron, hunter_state)).await? },
            async { spawn(web::serve(self.bind.address, self.bind.port, state)).await? },
        )?;

        Ok(())
    }
}
