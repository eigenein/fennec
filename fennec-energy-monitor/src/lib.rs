mod homewizard;
pub mod modbus;
mod result;

use tracing::info;
use tracing_subscriber::{
    fmt::{format::Pretty, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
use tracing_web::{MakeConsoleWriter, performance_layer};
use worker::{Date, Env, ScheduleContext, ScheduledEvent, wasm_bindgen::JsValue};
use worker_macros::event;

use crate::{homewizard::PowerMeasurement, result::Result};

#[event(start)]
fn started() {
    let format_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeConsoleWriter);
    let performance_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry().with(format_layer).with(performance_layer).init();
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _context: ScheduleContext) {
    try_scheduled(env).await.expect("the run has failed");
}

async fn try_scheduled(env: Env) -> Result {
    let (p1_measurement, battery_measurement, battery_status) = {
        let p1_client = homewizard::Client(env.service("p1")?);
        let battery_meter_client = homewizard::Client(env.service("batteryMeter")?);
        let modbus_client = modbus::Client(env.service("battery")?);
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

    let database = env.d1("fennec")?;

    // language=sqlite
    let query = r"
        INSERT INTO meters (
            timestamp,
            p1_import_kwh,
            p1_export_kwh,
            battery_import_kwh,
            battery_export_kwh,
            battery_residual_energy_kwh
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
    ";
    database
        .prepare(query)
        .bind(&[
            JsValue::from(u32::try_from(Date::now().as_millis() / 1000)?),
            JsValue::from(f64::from(p1_measurement.total_power_import)),
            JsValue::from(f64::from(p1_measurement.total_power_export)),
            JsValue::from(f64::from(battery_measurement.total_power_import)),
            JsValue::from(f64::from(battery_measurement.total_power_export)),
            JsValue::from(f64::from(battery_status.residual_energy())),
        ])?
        .run()
        .await?;

    Ok(())
}
