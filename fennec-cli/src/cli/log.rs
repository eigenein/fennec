use std::sync::{Arc, Mutex};

use bon::Builder;
use tokio::try_join;

use crate::{
    cli::connection::Connections,
    cron::CronSchedule,
    db::{Measurement, power},
    prelude::*,
    web::state::SystemState,
};

#[derive(Builder)]
pub struct Logger {
    connections: Connections,
    system_state: Arc<Mutex<SystemState<()>>>,
}

impl Logger {
    pub async fn run(self, schedule: CronSchedule) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *self.system_state.lock().unwrap() = match self.run_once().await {
                Ok(logger_state) => SystemState::ok(logger_state),
                Err(error) => SystemState::Err(error),
            };
        }
    }

    async fn run_once(&self) -> Result {
        let (battery_state, grid_metrics) = try_join!(
            self.connections.battery.read_state(),
            self.connections.grid_measurement.get_measurement()
        )?;
        power::Measurement::builder()
            .net_deficit(grid_metrics.active_power + battery_state.battery_active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build()
            .insert_into(&self.connections.db)
            .await?;
        Ok(())
    }
}
