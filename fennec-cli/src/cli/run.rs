use std::sync::{Arc, RwLock};

use clap::Parser;
use tokio::{spawn, try_join};

use crate::{
    battery::WorkingMode,
    cli::{
        battery::BatteryArgs,
        connection::ConnectionArgs,
        hunt::Hunter,
        log::Logger,
        web::BindArgs,
    },
    cron::CronSchedule,
    db::power,
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

    #[clap(long, env = "ENERGY_PROFILE_HALF_LIFE", default_value = "14d")]
    energy_profile_half_life: humantime::Duration,

    /// Do not push schedule to the device, dry run.
    #[clap(long, alias = "scout", env = "DRY_RUN")]
    dry_run: bool,
}

impl RunArgs {
    pub async fn run(self) -> Result {
        let battery_power_limits = self.battery.power_limits;

        let connections = self.connections.connect().await?;
        connections.db.set_expiration_time::<power::Measurement>(self.power_log_ttl.into()).await?;

        let mut logger = Logger::new(
            connections.clone(),
            battery_power_limits,
            HalfLife::new(self.energy_profile_half_life.into()),
        )
        .await?;
        let mut hunter = Hunter::builder()
            .connections(connections.clone())
            .working_modes(self.working_modes.iter().copied().collect())
            .energy_provider(self.energy_provider)
            .battery_args(self.battery)
            .quantum(self.quantum)
            .scout(self.dry_run)
            .build();

        // Run the first iteration at startup immediately in a fallible manner:
        let logger_state = Arc::new(RwLock::new(logger.run_once().await?));
        let hunter_state = Arc::new(RwLock::new(hunter.run_once().await?));
        let state =
            web::application::State { logger: logger_state.clone(), hunter: hunter_state.clone() };

        try_join!(
            async { spawn(logger.run_forever(self.logger_cron, logger_state)).await? },
            async { spawn(hunter.run_forever(self.optimizer_cron, hunter_state)).await? },
            async { spawn(web::serve(self.bind.address, self.bind.port, state)).await? },
        )?;

        connections.db.shutdown().await;
        Ok(())
    }
}
