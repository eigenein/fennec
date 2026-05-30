use std::sync::Arc;

use clap::Parser;
use tokio::{spawn, sync::RwLock, try_join};

use crate::{
    battery::WorkingMode,
    cli::{battery::BatteryArgs, connection::ConnectionArgs, hunter, logger, web::BindArgs},
    cron::CronSchedule,
    energy,
    math::smoothing::HalfLife,
    prelude::*,
    quantity::energy::WattHours,
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

    #[clap(long = "quantum-watthours", env = "QUANTUM_WATTHOURS", default_value = "1")]
    quantum: WattHours,

    /// Half-life for exponential moving average when learning
    /// the energy balance and battery parameters.
    #[clap(long, env = "LEARNING_HALF_LIFE", default_value = "14d")]
    learning_half_life: humantime::Duration,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    dry_run: bool,
}

impl RunArgs {
    pub async fn run(self) -> Result {
        let battery_power_limits = self.battery.power_limits;
        let connections = self.connections.connect()?;

        let logger_runner = logger::Args::builder()
            .connections(connections.clone())
            .battery_power_limits(battery_power_limits)
            .learning_half_life(HalfLife::new(self.learning_half_life))
            .build()
            .start()
            .await?;
        let hunter_runner = hunter::Runner::builder()
            .connections(connections.clone())
            .working_modes(self.working_modes.iter().copied().collect())
            .energy_provider(self.energy_provider)
            .battery_args(self.battery)
            .quantum(self.quantum)
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
