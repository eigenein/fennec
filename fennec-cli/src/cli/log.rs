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
}

impl Logger {
    pub async fn run_forever(
        self,
        schedule: CronSchedule,
        system_state: Arc<Mutex<SystemState<()>>>,
    ) -> Result {
        let mut cron = schedule.start();
        loop {
            cron.wait_until_next().await?;
            *system_state.lock().unwrap() = self.run_once_stateful().await;
        }
    }

    async fn run_once_stateful(&self) -> SystemState<()> {
        match self.run_once().await {
            Ok(logger_state) => SystemState::ok(logger_state),
            Err(error) => {
                error!("logger iteration failed: {error:#}");
                SystemState::Err(error)
            }
        }
    }

    async fn run_once(&self) -> Result {
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
        Ok(())
    }
}
