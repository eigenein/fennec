use std::time::Duration;

use bon::Builder;
use clap::Parser;
use reqwest::Url;
use tokio::time::sleep;

use crate::{
    api::{heartbeat, homewizard, modbus::foxess::EnergyStateClients},
    cli::{battery::BatteryEnergyStateUrls, db::DbArgs},
    db::{Db, TimeSeries, battery, consumption, state::BatteryResidualEnergy},
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
        let battery_meter_client = homewizard::Client::new(self.battery_energy_meter_url.clone())?;

        let battery_logger = BatteryLogger::builder()
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.battery_heartbeat_url.clone()))
            .interval(self.battery_polling_interval)
            .energy_state_clients(self.battery_energy_state_urls.connect().await?)
            .meter_client(battery_meter_client.clone())
            .build();
        let consumption_logger = ConsumptionLogger::builder()
            .interval(self.meter_polling_interval)
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.consumption_heartbeat_url.clone()))
            .total_meter_client(homewizard::Client::new(self.total_energy_meter_url.clone())?)
            .battery_meter_client(battery_meter_client)
            .build();

        let result = tokio::try_join!(battery_logger.run(), consumption_logger.run());
        db.shutdown().await;
        result?;
        Ok(())
    }
}

#[derive(Builder)]
struct ConsumptionLogger {
    battery_meter_client: homewizard::Client,
    total_meter_client: homewizard::Client,
    db: Db,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl ConsumptionLogger {
    async fn run(self) -> Result {
        loop {
            let (total_metrics, battery_metrics) = tokio::try_join!(
                self.total_meter_client.get_measurement(),
                self.battery_meter_client.get_measurement(),
            )?;
            let entry = consumption::LogEntry::builder()
                .net_deficit(total_metrics.net_import() - battery_metrics.net_import())
                .build();
            info!(deficit = ?entry.net_deficit, "consumption log");
            entry.insert_into(&self.db).await?;
            self.heartbeat.send().await;
            sleep(self.interval).await;
        }
    }
}

#[derive(Builder)]
struct BatteryLogger {
    energy_state_clients: EnergyStateClients,
    meter_client: homewizard::Client,
    db: Db,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl BatteryLogger {
    async fn run(self) -> Result {
        loop {
            let battery_state = self.energy_state_clients.read().await?;
            info!(residual = ?battery_state.residual_millis(), "battery state");
            let last_known_residual_energy = self
                .db
                .set_application_state(&BatteryResidualEnergy::from(
                    battery_state.residual_millis(),
                ))
                .await?
                .map(MilliwattHours::from);
            if let Some(last_known_residual_energy) = last_known_residual_energy
                && (last_known_residual_energy != battery_state.residual_millis())
            {
                let entry = battery::LogEntry::builder()
                    .residual_energy(battery_state.residual_millis())
                    .metrics(self.meter_client.get_measurement().await?)
                    .build();
                info!(residual = ?entry.residual_energy, import = ?entry.metrics.import, export = ?entry.metrics.export, "battery log");
                entry.insert_into(&self.db).await?;
            }
            self.heartbeat.send().await;
            sleep(self.interval).await;
        }
    }
}
