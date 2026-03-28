use std::time::Duration;

use bon::{Builder, bon};
use clap::Parser;
use tokio::{
    time::{MissedTickBehavior, interval},
    try_join,
};

use crate::{
    api::homewizard::EnergyMetrics,
    battery,
    cli::connection::{ConnectionArgs, Connections},
    db::{Measurement, power},
    prelude::*,
};

#[derive(Parser)]
pub struct LogArgs {
    #[clap(flatten)]
    connections: ConnectionArgs,

    #[clap(long, env = "BATTERY_POLLING_INTERVAL", default_value = "5s")]
    battery_polling_interval: humantime::Duration,

    #[clap(long, env = "METER_POLLING_INTERVAL", default_value = "1min")]
    meter_polling_interval: humantime::Duration,

    #[clap(long = "power-log-ttl", env = "POWER_LOG_TTL", default_value = "14days")]
    power_log_ttl: humantime::Duration,

    #[clap(long = "battery-log-ttl", env = "BATTERY_LOG_TTL", default_value = "14days")]
    battery_log_ttl: humantime::Duration,
}

impl LogArgs {
    pub async fn run(self) -> Result {
        let connections = self.connections.connect().await?;

        // FIXME: db.set_expiration_time::<battery::Measurement>(self.battery_log_ttl.into()).await?;
        connections.db.set_expiration_time::<power::Measurement>(self.power_log_ttl.into()).await?;

        let result = Logger::builder()
            .connections(connections.clone())
            .interval(self.battery_polling_interval)
            .build()
            .run()
            .await;

        connections.db.shutdown().await;
        result
    }
}

/// TODO: just move the loop.
#[derive(Builder)]
struct Logger {
    connections: Connections,

    #[builder(into)]
    interval: Duration,
}

impl Logger {
    async fn run(self) -> Result {
        let mut interval = interval(self.interval);
        interval.reset_after(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let (battery_state, grid_metrics) = try_join!(
                self.connections.battery.read_state(),
                self.connections.grid_measurement.get_measurement()
            )?;
            self.log_active_power()
                .grid_metrics(&grid_metrics)
                .battery_state(&battery_state)
                .call()
                .await?;
        }
    }
}

#[bon]
impl Logger {
    #[builder]
    async fn log_active_power(
        &self,
        grid_metrics: &EnergyMetrics,
        battery_state: &battery::State,
    ) -> Result {
        power::Measurement::builder()
            .net_deficit(grid_metrics.active_power + battery_state.battery_active_power)
            .eps_active_power(battery_state.eps_active_power)
            .build()
            .insert_into(&self.connections.db)
            .await
    }
}
