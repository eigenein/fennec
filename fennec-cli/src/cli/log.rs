use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::time::sleep;

use crate::{
    api::{homewizard, modbus},
    cli::LogArgs,
    db::{
        Db,
        battery_log::{BatteryLog, BatteryLogs},
        state::{BatteryResidualEnergy, States},
    },
    prelude::*,
    quantity::energy::MilliwattHours,
};

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

    let mut battery = modbus::Client::connect(&args.battery_connection).await?;
    let db = Db::with_uri(&args.db.uri).await?;

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
