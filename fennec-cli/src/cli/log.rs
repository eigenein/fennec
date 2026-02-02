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

pub async fn log(args: LogArgs) -> Result {
    // TODO: implement proper signal handling with cancelling the `sleep` call.
    let should_terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&should_terminate))?;
    tokio::try_join!(
        log_battery(&args, Arc::clone(&should_terminate)),
        log_energy_meter(&args, should_terminate)
    )?;
    Ok(())
}

async fn log_battery(args: &LogArgs, should_terminate: Arc<AtomicBool>) -> Result {
    let polling_interval: Duration = args.battery_polling_interval();
    let battery_energy_meter = homewizard::Client::new(args.battery_energy_meter_url.clone())?;

    info!("verifying energy meter connection…");
    let _ = battery_energy_meter.get_measurement().await?;

    let mut battery = args.battery_connection.connect().await?;
    let db = args.db.connect().await?;

    while !should_terminate.load(Ordering::Relaxed) {
        let battery_state = battery.read_energy_state(args.battery_registers).await?;
        let last_known_residual_energy = States::from(&db)
            .set(&BatteryResidualEnergy::from(battery_state.residual_millis()))
            .await?
            .map(MilliwattHours::from);
        if let Some(last_known_residual_energy) = last_known_residual_energy
            && (last_known_residual_energy != battery_state.residual_millis())
        {
            let metrics = battery_energy_meter.get_measurement().await?;
            let log = BatteryLog::builder()
                .residual_energy(battery_state.residual_millis())
                .metrics(metrics)
                .build();
            BatteryLogs::from(&db).insert(&log).await?;
        }

        args.battery_heartbeat.send().await;
        sleep(polling_interval).await;
    }

    Ok(())
}

async fn log_energy_meter(args: &LogArgs, should_terminate: Arc<AtomicBool>) -> Result {
    let polling_interval: Duration = args.meter_polling_interval();
    let total_energy_meter = homewizard::Client::new(args.total_energy_meter_url.clone())?;

    while !should_terminate.load(Ordering::Relaxed) {
        let metrics = total_energy_meter.get_measurement().await?;
        // TODO: heartbeat.
        sleep(polling_interval).await;
    }

    Ok(())
}
