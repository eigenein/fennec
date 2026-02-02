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
    api::homewizard,
    cli::{
        battery::{BatteryConnectionArgs, BatteryEnergyStateRegisters},
        db::DbArgs,
        heartbeat::HeartbeatArgs,
    },
    db::{
        battery_log::{BatteryLog, BatteryLogs},
        meter_log::MeterLog,
        state::{BatteryResidualEnergy, States},
    },
    prelude::*,
    quantity::energy::MilliwattHours,
};

#[derive(Parser)]
pub struct LogArgs {
    #[clap(long, env = "BATTERY_POLLING_INTERVAL", default_value = "5s")]
    battery_polling_interval: humantime::Duration,

    #[clap(long, env = "METER_POLLING_INTERVAL", default_value = "5min")]
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

    #[clap(flatten)]
    battery_heartbeat: HeartbeatArgs,
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
        tokio::try_join!(
            self.log_battery(Arc::clone(&should_terminate)),
            self.log_energy_meter(should_terminate)
        )?;
        Ok(())
    }

    async fn log_battery(&self, should_terminate: Arc<AtomicBool>) -> Result {
        let polling_interval: Duration = self.battery_polling_interval();
        let battery_energy_meter = homewizard::Client::new(self.battery_energy_meter_url.clone())?;

        info!("verifying energy meter connection…");
        let _ = battery_energy_meter.get_measurement().await?;

        let mut battery = self.battery_connection.connect().await?;
        let db = self.db.connect().await?;

        while !should_terminate.load(Ordering::Relaxed) {
            let battery_state = battery.read_energy_state(self.battery_registers).await?;
            let last_known_residual_energy = States::from(&db)
                .set(&BatteryResidualEnergy::from(battery_state.residual_millis()))
                .await?
                .map(MilliwattHours::from);
            if let Some(last_known_residual_energy) = last_known_residual_energy
                && (last_known_residual_energy != battery_state.residual_millis())
            {
                let log = BatteryLog::builder()
                    .residual_energy(battery_state.residual_millis())
                    .metrics(battery_energy_meter.get_measurement().await?)
                    .build();
                BatteryLogs::from(&db).insert(&log).await?;
            }

            self.battery_heartbeat.send().await;
            sleep(polling_interval).await;
        }

        Ok(())
    }

    async fn log_energy_meter(&self, should_terminate: Arc<AtomicBool>) -> Result {
        let polling_interval: Duration = self.meter_polling_interval();
        let total_energy_meter = homewizard::Client::new(self.total_energy_meter_url.clone())?;

        while !should_terminate.load(Ordering::Relaxed) {
            let log =
                MeterLog::builder().metrics(total_energy_meter.get_measurement().await?).build();
            // TODO: heartbeat.
            sleep(polling_interval).await;
        }

        Ok(())
    }
}
