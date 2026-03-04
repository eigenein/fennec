use std::time::Duration;

use bon::Builder;
use clap::Parser;
use reqwest::Url;
use tokio::{
    time::{MissedTickBehavior, interval},
    try_join,
};

use crate::{
    api::{heartbeat, homewizard, modbus::foxess::MQ2200},
    cli::{battery::BatteryConnectionArgs, db::DbArgs},
    db::{Db, Measurement, battery, power, state::BatteryResidualEnergy},
    prelude::*,
    quantity::energy::MilliwattHours,
};

#[derive(Parser)]
pub struct LogArgs {
    #[clap(long, env = "BATTERY_POLLING_INTERVAL", default_value = "5s")]
    battery_polling_interval: humantime::Duration,

    #[clap(long, env = "METER_POLLING_INTERVAL", default_value = "1min")]
    meter_polling_interval: humantime::Duration,

    #[clap(long, env = "TOTAL_ENERGY_METER_URL")]
    total_energy_meter_url: Url,

    #[clap(long, env = "BATTERY_ENERGY_METER_URL")]
    battery_energy_meter_url: Url,

    #[clap(long, env = "MEASUREMENT_EXPIRATION_TIME", default_value = "14days")]
    measurement_expiration_time: humantime::Duration,

    #[clap(flatten)]
    db: DbArgs,

    #[clap(flatten)]
    battery_connection: BatteryConnectionArgs,

    #[clap(long = "heartbeat-url", env = "LOG_HEARTBEAT_URL")]
    heartbeat_url: Option<Url>,
}

impl LogArgs {
    pub async fn run(self) -> Result {
        let db = self.db.connect().await?;
        db.set_expiration_time(self.measurement_expiration_time.into()).await?;
        let grid_meter_client = homewizard::Client::new(self.total_energy_meter_url)?;
        let battery_meter_client = homewizard::Client::new(self.battery_energy_meter_url)?;

        let result = Logger::builder()
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.heartbeat_url.clone()))
            .interval(self.battery_polling_interval)
            .battery_client(self.battery_connection.connect().await?)
            .battery_meter_client(battery_meter_client)
            .grid_meter_client(grid_meter_client)
            .build()
            .run()
            .await;
        db.shutdown().await;
        result?;
        Ok(())
    }
}

/// TODO: just move the loop.
#[derive(Builder)]
struct Logger {
    battery_client: MQ2200,
    battery_meter_client: homewizard::Client,
    grid_meter_client: homewizard::Client,
    db: Db,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl Logger {
    async fn run(mut self) -> Result {
        let mut interval = interval(self.interval);
        interval.reset_after(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let (battery_state, battery_metrics, grid_metrics) = try_join!(
                self.battery_client.read_energy_state(),
                self.battery_meter_client.get_measurement(),
                self.grid_meter_client.get_measurement()
            )?;

            let previous_residual_energy = self
                .db
                .set_application_state(&BatteryResidualEnergy::from(
                    battery_state.residual_millis(),
                ))
                .await?
                .map(MilliwattHours::from);
            if let Some(last_known_residual_energy) = previous_residual_energy
                && (last_known_residual_energy != battery_state.residual_millis())
            {
                battery::Measurement::builder()
                    .legacy_residual_energy(battery_state.residual_millis())
                    .residual_energy(battery_state.residual_millis())
                    .legacy_import(battery_metrics.import)
                    .import(battery_metrics.import)
                    .legacy_export(battery_metrics.export)
                    .export(battery_metrics.export)
                    .build()
                    .insert_into(&self.db)
                    .await?;
            }

            power::Measurement::builder()
                .net_power(grid_metrics.active_power - battery_metrics.active_power)
                .build()
                .insert_into(&self.db)
                .await?;

            self.heartbeat.send().await;
        }
    }
}
