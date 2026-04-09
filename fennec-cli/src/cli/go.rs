use std::net::IpAddr;

use clap::Parser;
use tokio::{spawn, try_join};

use crate::{
    cli::{hunt::HuntSharedArgs, log::Logger},
    cron::CronSchedule,
    db::power,
    prelude::*,
    web,
};

#[derive(Parser)]
pub struct GoArgs {
    #[clap(flatten)]
    shared: HuntSharedArgs,

    #[clap(long = "logger-cron", env = "LOGGER_CRON", default_value = "*/5 * * * * *")]
    logger_cron: CronSchedule,

    #[clap(long = "optimizer-cron", env = "OPTIMIZER_CRON", default_value = "0 */15 * * * *")]
    optimizer_cron: CronSchedule,

    #[clap(long = "power-log-ttl", env = "POWER_LOG_TTL", default_value = "14days")]
    power_log_ttl: humantime::Duration,

    #[clap(long = "bind-address", env = "BIND_ADDRESS", default_value = "::")]
    bind_address: IpAddr,

    #[clap(long = "bind-port", env = "BIND_PORT", default_value = "80")]
    bind_port: u16,
}

impl GoArgs {
    pub async fn run(self) -> Result {
        let (connections, hunter) = self.shared.hunter().await?;
        connections.db.set_expiration_time::<power::Measurement>(self.power_log_ttl.into()).await?;

        let logger = Logger::builder().connections(connections.clone()).build();

        // Run the first iteration at startup immediately in a fallible manner:
        let hunter_state = web::application::Component::now(hunter.run_once().await?);
        let logger_state = web::application::Component::now(logger.run_once().await?);
        let state =
            web::application::State { logger: logger_state.clone(), hunter: hunter_state.clone() };

        try_join!(
            async { spawn(logger.run_forever(self.logger_cron, logger_state)).await? },
            async { spawn(hunter.run_forever(self.optimizer_cron, hunter_state)).await? },
            async { spawn(web::serve(self.bind_address, self.bind_port, state)).await? },
        )?;

        connections.db.shutdown().await;
        Ok(())
    }
}
