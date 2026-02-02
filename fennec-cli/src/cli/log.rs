use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use clap::Parser;
use reqwest::Url;
use tokio::time::sleep;

use crate::{
    api::{heartbeat, homewizard},
    cli::{
        battery::{BatteryConnectionArgs, BatteryEnergyStateRegisters},
        db::DbArgs,
    },
    db::{
        Db,
        battery::BatteryLog,
        consumption::ConsumptionLog,
        log::Log,
        state::{BatteryResidualEnergy, States},
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
    fn battery_polling_interval(&self) -> Duration {
        self.battery_polling_interval.into()
    }

    fn meter_polling_interval(&self) -> Duration {
        self.meter_polling_interval.into()
    }
}

impl LogArgs {
    pub async fn log(self) -> Result {
        // TODO: implement proper signal handling with cancelling the `sleep` call.
        let should_terminate = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&should_terminate))?;

        let db = self.db.connect().await?;
        let battery_meter = homewizard::Client::new(self.battery_energy_meter_url.clone())?;

        tokio::try_join!(
            self.log_battery(battery_meter.clone(), db.clone(), Arc::clone(&should_terminate)),
            self.log_consumption(battery_meter, db, should_terminate)
        )?;
        Ok(())
    }

    async fn log_battery(
        &self,
        battery_meter: homewizard::Client,
        db: Db,
        should_terminate: Arc<AtomicBool>,
    ) -> Result {
        let polling_interval: Duration = self.battery_polling_interval();
        let heartbeat = heartbeat::Client::new(self.battery_heartbeat_url.clone());
        let mut battery = self.battery_connection.connect().await?;

        while !should_terminate.load(Ordering::Relaxed) {
            let battery_state = battery.read_energy_state(self.battery_registers).await?;
            let last_known_residual_energy = States::from(&db)
                .set(&BatteryResidualEnergy::from(battery_state.residual_millis()))
                .await?
                .map(MilliwattHours::from);
            if let Some(last_known_residual_energy) = last_known_residual_energy
                && (last_known_residual_energy != battery_state.residual_millis())
            {
                BatteryLog::builder()
                    .residual_energy(battery_state.residual_millis())
                    .metrics(battery_meter.get_measurement().await?)
                    .build()
                    .insert_into(&db)
                    .await?;
            }

            heartbeat.send().await;
            sleep(polling_interval).await;
        }

        Ok(())
    }

    async fn log_consumption(
        &self,
        battery_meter: homewizard::Client,
        db: Db,
        should_terminate: Arc<AtomicBool>,
    ) -> Result {
        let polling_interval: Duration = self.meter_polling_interval();
        let heartbeat = heartbeat::Client::new(self.consumption_heartbeat_url.clone());
        let total_meter = homewizard::Client::new(self.total_energy_meter_url.clone())?;

        while !should_terminate.load(Ordering::Relaxed) {
            let (total_metrics, battery_metrics) =
                tokio::try_join!(total_meter.get_measurement(), battery_meter.get_measurement())?;
            ConsumptionLog::builder()
                .net(total_metrics.net_consumption() - battery_metrics.net_consumption())
                .build()
                .insert_into(&db)
                .await?;
            heartbeat.send().await;
            sleep(polling_interval).await;
        }

        Ok(())
    }
}
