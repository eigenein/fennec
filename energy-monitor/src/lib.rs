mod home_wizard;
mod result;

use tracing::info;
use tracing_subscriber::{fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_web::{MakeConsoleWriter, performance_layer};
use worker::{Env, ScheduleContext, ScheduledEvent};
use worker_macros::{event, send};

use crate::{
    home_wizard::{Client, PowerMeasurement},
    result::Result,
};

#[event(start)]
fn started() {
    let format_layer =
        tracing_subscriber::fmt::layer().json().without_time().with_writer(MakeConsoleWriter);
    let performance_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry().with(format_layer).with(performance_layer).init();
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _context: ScheduleContext) {
    try_scheduled(env).await.unwrap();
}

async fn try_scheduled(env: Env) -> Result {
    let p1_measurement: PowerMeasurement =
        Client(env.service("p1Service")?).get_measurement().await?;
    info!(%p1_measurement.total_power_import, %p1_measurement.total_power_export);

    let battery_measurement: PowerMeasurement =
        Client(env.service("batteryMeterService")?).get_measurement().await?;
    info!(%battery_measurement.total_power_import, %battery_measurement.total_power_export);

    Ok(())
}
