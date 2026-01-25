use turso::Connection;

use crate::{db::battery_log::BatteryLog, prelude::*};

pub struct BatteryLogs<'c>(pub &'c Connection);

impl BatteryLogs<'_> {
    #[instrument(skip_all, fields(residual = ?log.residual_energy))]
    pub async fn insert(&self, log: &BatteryLog) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO battery_logs (
                timestamp_millis,
                residual_energy_kwh,
                import_kwh,
                export_kwh
            ) VALUES (?1, ?2, ?3, ?4)
        ";

        info!("inserting the battery_log…");
        self.0
            .prepare_cached(SQL)
            .await?
            .execute((
                log.timestamp.timestamp_millis(),
                log.residual_energy,
                log.meter_measurement.import,
                log.meter_measurement.export,
            ))
            .await?;
        Ok(())
    }
}
