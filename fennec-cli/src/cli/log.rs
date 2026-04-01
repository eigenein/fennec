use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use backon::{ConstantBuilder, Retryable};
use bon::Builder;
use tokio::try_join;

use crate::{
    cli::connection::Connections,
    cron::CronSchedule,
    db::{Measurement, power},
    prelude::*,
    state::LoggerState,
    web::state::SystemState,
};

/// Battery state and power meter logger.
#[derive(Builder)]
pub struct Logger {
    connections: Connections,
}

impl Logger {
    const BACKOFF: ConstantBuilder = ConstantBuilder::new().with_delay(Duration::from_secs(1));

    pub async fn run_forever(
        self,
        schedule: CronSchedule,
        system_state: Arc<RwLock<SystemState<LoggerState>>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            let run_once = || async {
                self.run_once()
                    .await
                    .inspect_err(|error| error!("logger iteration failed: {error:#}"))
            };
            *system_state.write().unwrap() = run_once.retry(Self::BACKOFF).await?.into();
        }
    }

    /// Run a single logging iteration.
    ///
    /// We don't care about retries here because the logger is supposed to run frequently anyway.
    pub async fn run_once(&self) -> Result<LoggerState> {
        let (battery_state, grid_metrics) = try_join!(
            async { self.connections.battery.lock().await.read_state().await },
            self.connections.grid_measurement.get_measurement()
        )?;
        power::Measurement::builder()
            .net_deficit(grid_metrics.active_power + battery_state.battery_active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build()
            .insert_into(&self.connections.db)
            .await?;
        Ok(LoggerState { battery: battery_state })
    }
}
