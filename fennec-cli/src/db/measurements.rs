use bon::Builder;
use chrono::Utc;
use turso::Connection;

use crate::{api::homewizard::MeterMeasurement, prelude::*, quantity::energy::KilowattHours};

#[must_use]
pub struct Measurements<'c>(pub &'c Connection);

impl Measurements<'_> {
    pub async fn upsert(&self, measurement: &Measurement) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO measurements (
                timestamp_millis,
                total_import_kwh,
                total_export_kwh,
                battery_import_kwh,
                battery_export_kwh,
                battery_energy_kwh
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ";

        info!("Upserting the measurement…");
        self.0
            .prepare_cached(SQL)
            .await?
            .execute((
                Utc::now().timestamp_millis(),
                measurement.total.import,
                measurement.total.export,
                measurement.battery.import,
                measurement.battery.export,
                measurement.residual_energy,
            ))
            .await?;
        Ok(())
    }
}

#[derive(Builder)]
pub struct Measurement {
    pub total: MeterMeasurement,
    pub battery: MeterMeasurement,
    pub residual_energy: KilowattHours,
}
