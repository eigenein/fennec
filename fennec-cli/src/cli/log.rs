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
            *system_state.write().unwrap() =
                self.run_once().await.context("the logger iteration has failed")?.into();
        }
    }

    /// Run a single logging iteration.
    pub async fn run_once(&self) -> Result<LoggerState> {
        let read_state = || async {
            // Retry them together to ensure the measurements are in sync.
            try_join!(
                async { self.connections.battery.lock().await.read_state().await },
                self.connections.grid_measurement.get_measurement()
            )
        };
        let (battery_state, grid_metrics) =
            read_state.retry(Self::BACKOFF).notify(log_error).await?;
        let battery_measurement = power::BatteryMeasurement::builder()
            .residual_energy(battery_state.residual_energy())
            .active_power(battery_state.active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build();
        power::Measurement::builder()
            .net_deficit(grid_metrics.active_power + battery_state.active_power)
            .eps_active_power(battery_state.eps_active_power)
            .battery(battery_measurement)
            .build()
            .insert_into(&self.connections.db)
            .await?;
        Ok(LoggerState { battery: battery_state })
    }
}
