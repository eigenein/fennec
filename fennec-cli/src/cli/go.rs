use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};

use clap::Parser;
use tokio::{spawn, try_join};

use crate::{
    cli::{hunt::HuntSharedArgs, log::Logger},
    cron::CronSchedule,
    db::power,
    prelude::*,
    web,
    web::state::ApplicationState,
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

        let (logger_result, hunter_result, web_result) = {
            let logger = Logger::builder().connections(connections.clone()).build();
            // Run the first iteration at startup immediately in a fallible manner:
            let application_state = ApplicationState {
                logger: Arc::new(RwLock::new(logger.run_once().await?.into())),
                hunter: Arc::new(RwLock::new(hunter.run_once().await?.into())),
            };
            try_join!(
                spawn(logger.run_forever(self.logger_cron, application_state.logger.clone())),
                spawn(hunter.run_forever(self.optimizer_cron, application_state.hunter.clone())),
                spawn(web::serve(self.bind_address, self.bind_port, application_state)),
            )?
        };

        connections.db.shutdown().await;
        logger_result.and(hunter_result).and(web_result)
    }
}
