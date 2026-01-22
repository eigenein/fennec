use chrono::Utc;
use turso::Connection;

use crate::{api::homewizard::PowerMeasurement, prelude::*, quantity::energy::KilowattHours};

#[must_use]
pub struct Measurements<'c>(pub &'c Connection);

impl Measurements<'_> {
    pub async fn upsert(
        &self,
        total: PowerMeasurement,
        battery: PowerMeasurement,
        residual: KilowattHours,
    ) -> Result {
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
        self.0
            .prepare_cached(SQL)
            .await?
            .execute((
                Utc::now().timestamp_millis(),
                total.import,
                total.export,
                battery.import,
                battery.export,
                residual,
            ))
            .await?;
        Ok(())
    }
}
