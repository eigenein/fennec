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
    schedule: CronSchedule,
    system_state: Arc<Mutex<SystemState<()>>>,
}

impl Logger {
    pub async fn run(self) -> Result {
        let mut cron = self.schedule.start();

        loop {
            cron.wait_until_next().await?;

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

            // FIXME: handle «error» state.
            *self.system_state.lock().unwrap() = SystemState::ok(());
        }
    }
}
