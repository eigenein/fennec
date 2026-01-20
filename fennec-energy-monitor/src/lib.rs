mod homewizard;
pub mod modbus;
mod result;
mod timer;

use tracing::info;
use tracing_subscriber::{fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_web::{MakeConsoleWriter, performance_layer};
use worker::{Env, ScheduleContext, ScheduledEvent};
use worker_macros::event;

use crate::{homewizard::PowerMeasurement, result::Result, timer::WorkerFormatTime};

#[event(start)]
fn started() {
    let format_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_timer(WorkerFormatTime)
        .with_writer(MakeConsoleWriter);
    let performance_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry().with(format_layer).with(performance_layer).init();
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _context: ScheduleContext) {
    try_scheduled(env).await.unwrap();
}

async fn try_scheduled(env: Env) -> Result {
    let (p1_measurement, battery_measurement, battery_status) = {
        let p1_client = homewizard::Client(env.service("p1Service")?);
        let battery_meter_client = homewizard::Client(env.service("batteryMeterService")?);
        let modbus_client = modbus::Client(env.service("batteryService")?);
        futures::try_join!(
            p1_client.get_measurement::<PowerMeasurement>(),
            battery_meter_client.get_measurement::<PowerMeasurement>(),
            modbus_client.get_battery_status(),
        )?
    };

    info!(%p1_measurement.total_power_import, %p1_measurement.total_power_export);
    info!(%battery_measurement.total_power_import, %battery_measurement.total_power_export);
    info!(
        state_of_charge = battery_status.state_of_charge,
        state_of_health = battery_status.state_of_health,
        design_capacity = ?battery_status.design_capacity,
    );

    Ok(())
}
