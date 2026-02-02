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
    db::{Db, battery_log::BatteryLog, state::BatteryResidualEnergy},
    prelude::*,
    quantity::energy::MilliwattHours,
};

/// TODO: separate loops and intervals for battery and P1 loggers.
pub async fn log(args: LogArgs) -> Result {
    // TODO: this one should be independently fallible:
    // let total_energy_meter = homewizard::Client::new(args.total_energy_meter_url)?;

    let polling_interval: Duration = args.polling_interval();
    let battery_energy_meter = homewizard::Client::new(args.battery_energy_meter_url)?;
    let mut battery = modbus::Client::connect(&args.battery_connection).await?;
    let db = Db::with_uri(args.db.uri).await?;

    // TODO: implement proper signal handling with cancelling the `sleep` call.
    let should_terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&should_terminate))?;

    while !should_terminate.load(Ordering::Relaxed) {
        let (battery_measurement, battery_state) = {
            tokio::try_join!(
                battery_energy_meter.get_measurement(),
                battery.read_energy_state(args.battery_registers),
            )?
        };

        let mut db = db.start_session().await?;
        db.session().start_transaction().await?;
        let last_known_residual_energy =
            db.states().get::<BatteryResidualEnergy>().await?.map(MilliwattHours::from);
        if let Some(last_known_residual_energy) = last_known_residual_energy
            && (last_known_residual_energy != battery_state.residual_millis())
        {
            let battery_log = BatteryLog::builder()
                .residual_energy(battery_state.residual_millis())
                .meter(battery_measurement)
                .build();
            db.battery_logs().insert(&battery_log).await?;
        }
        db.states().upsert(&BatteryResidualEnergy::from(battery_state.residual_millis())).await?;
        db.session().commit_transaction().await?;

        args.heartbeat.send().await;
        sleep(polling_interval).await;
    }

    Ok(())
}
