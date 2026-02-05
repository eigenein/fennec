use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bon::Builder;
use clap::Parser;
use reqwest::Url;
use tokio::time::sleep;

use crate::{
    api::{heartbeat, homewizard, modbus},
    cli::{
        battery::{BatteryConnectionArgs, BatteryEnergyStateRegisters},
        db::DbArgs,
    },
    db::{
        Db,
        battery::BatteryLog,
        consumption::ConsumptionLog,
        log::Log,
        state::BatteryResidualEnergy,
    },
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
    battery_connection: BatteryConnectionArgs,

    #[clap(flatten)]
    battery_registers: BatteryEnergyStateRegisters,

    #[clap(long = "battery-heartbeat-url", env = "BATTERY_HEARTBEAT_URL")]
    battery_heartbeat_url: Option<Url>,

    #[clap(long = "consumption-heartbeat-url", env = "CONSUMPTION_HEARTBEAT_URL")]
    consumption_heartbeat_url: Option<Url>,
}

impl LogArgs {
    pub async fn run(self) -> Result {
        // TODO: implement proper signal handling with cancelling the `sleep` call.
        let should_terminate = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&should_terminate))?;

        let db = self.db.connect().await?;
        let battery_meter_client = homewizard::Client::new(self.battery_energy_meter_url.clone())?;

        let battery_logger = BatteryLogger::builder()
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.battery_heartbeat_url.clone()))
            .interval(self.battery_polling_interval)
            .modbus_client(self.battery_connection.connect().await?)
            .modbus_registers(self.battery_registers)
            .meter_client(battery_meter_client.clone())
            .should_terminate(Arc::clone(&should_terminate))
            .build();
        let consumption_logger = ConsumptionLogger::builder()
            .interval(self.meter_polling_interval)
            .db(db.clone())
            .heartbeat(heartbeat::Client::new(self.consumption_heartbeat_url.clone()))
            .total_meter_client(homewizard::Client::new(self.total_energy_meter_url.clone())?)
            .battery_meter_client(battery_meter_client)
            .should_terminate(should_terminate)
            .build();

        let result = tokio::try_join!(battery_logger.run(), consumption_logger.run());
        db.shutdown().await;
        result.map(|_| ())
    }
}

#[derive(Builder)]
struct ConsumptionLogger {
    battery_meter_client: homewizard::Client,
    total_meter_client: homewizard::Client,
    db: Db,
    should_terminate: Arc<AtomicBool>,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl ConsumptionLogger {
    async fn run(self) -> Result {
        while !self.should_terminate.load(Ordering::Relaxed) {
            let (total_metrics, battery_metrics) = tokio::try_join!(
                self.total_meter_client.get_measurement(),
                self.battery_meter_client.get_measurement()
            )?;
            ConsumptionLog::builder()
                .net(total_metrics.net_consumption() - battery_metrics.net_consumption())
                .build()
                .insert_into(&self.db)
                .await?;
            self.heartbeat.send().await;
            sleep(self.interval).await;
        }
        Ok(())
    }
}

#[derive(Builder)]
struct BatteryLogger {
    modbus_client: modbus::Client,
    modbus_registers: BatteryEnergyStateRegisters,
    meter_client: homewizard::Client,
    db: Db,
    should_terminate: Arc<AtomicBool>,
    heartbeat: heartbeat::Client,

    #[builder(into)]
    interval: Duration,
}

impl BatteryLogger {
    async fn run(mut self) -> Result {
        while !self.should_terminate.load(Ordering::Relaxed) {
            let battery_state = self.modbus_client.read_energy_state(self.modbus_registers).await?;
            let last_known_residual_energy = self
                .db
                .set_state(&BatteryResidualEnergy::from(battery_state.residual_millis()))
                .await?
                .map(MilliwattHours::from);
            if let Some(last_known_residual_energy) = last_known_residual_energy
                && (last_known_residual_energy != battery_state.residual_millis())
            {
                BatteryLog::builder()
                    .residual_energy(battery_state.residual_millis())
                    .metrics(self.meter_client.get_measurement().await?)
                    .build()
                    .insert_into(&self.db)
                    .await?;
            }
            self.heartbeat.send().await;
            sleep(self.interval).await;
        }
        Ok(())
    }
}
