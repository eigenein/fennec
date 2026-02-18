use std::time::Duration;

use bon::Builder;
use clap::Parser;
use reqwest::Url;
use tokio::{
    time::{MissedTickBehavior, interval},
    try_join,
};

use crate::{
    api::{heartbeat, homewizard, modbus::foxess::EnergyStateClients},
    cli::{battery::BatteryEnergyStateUrls, db::DbArgs},
    db::{Db, Measurement, battery, consumption, power, state::BatteryResidualEnergy},
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
    battery_energy_state_urls: BatteryEnergyStateUrls,

    #[clap(long = "battery-heartbeat-url", env = "BATTERY_HEARTBEAT_URL")]
    battery_heartbeat_url: Option<Url>,

    #[clap(long = "consumption-heartbeat-url", env = "CONSUMPTION_HEARTBEAT_URL")]
    consumption_heartbeat_url: Option<Url>,
}

impl LogArgs {
    pub async fn run(self) -> Result {
        let db = self.db.connect().await?;
        db.set_expiration_time(self.measurement_expiration_time.into()).await?;
        let grid_meter_client = homewizard::Client::new(self.total_energy_meter_url)?;
        let battery_meter_client = homewizard::Client::new(self.battery_energy_meter_url)?;

        let logger = Logger::builder()
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.battery_heartbeat_url.clone()))
            .interval(self.battery_polling_interval)
            .energy_state_clients(self.battery_energy_state_urls.connect().await?)
            .battery_meter_client(battery_meter_client.clone())
            .grid_meter_client(grid_meter_client.clone())
            .build();
        let legacy_consumption_logger = LegacyConsumptionLogger::builder()
            .interval(self.meter_polling_interval)
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.consumption_heartbeat_url.clone()))
            .grid_meter_client(grid_meter_client)
            .battery_meter_client(battery_meter_client)
            .build();

        let result = tokio::try_join!(logger.run(), legacy_consumption_logger.run());
        db.shutdown().await;
        result?;
        Ok(())
    }
}

#[derive(Builder)]
struct LegacyConsumptionLogger {
    battery_meter_client: homewizard::Client,
    grid_meter_client: homewizard::Client,
    db: Db,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl LegacyConsumptionLogger {
    async fn run(self) -> Result {
        let mut interval = interval(self.interval);
        interval.reset_after(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let (grid_metrics, battery_metrics) = tokio::try_join!(
                self.grid_meter_client.get_measurement(),
                self.battery_meter_client.get_measurement(),
            )?;
            consumption::Measurement::builder()
                .net_deficit(grid_metrics.net_import() - battery_metrics.net_import())
                .build()
                .insert_into(&self.db)
                .await?;

            self.heartbeat.send().await;
        }
    }
}

#[derive(Builder)]
struct Logger {
    energy_state_clients: EnergyStateClients,
    battery_meter_client: homewizard::Client,
    grid_meter_client: homewizard::Client,
    db: Db,
    heartbeat: heartbeat::Client,

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

            let (battery_state, battery_metrics, grid_metrics) = try_join!(
                self.energy_state_clients.read(),
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
                    .residual_energy(battery_state.residual_millis())
                    .import(battery_metrics.import)
                    .export(battery_metrics.export)
                    .build()
                    .insert_into(&self.db)
                    .await?;
            }

            power::Measurement::builder()
                .net(grid_metrics.active_power - battery_metrics.active_power)
                .build()
                .insert_into(&self.db)
                .await?;

            self.heartbeat.send().await;
        }
    }
}
