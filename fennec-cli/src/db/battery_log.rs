use async_stream::try_stream;
use bon::Builder;
use chrono::{DateTime, Local, TimeZone};
use futures_core::Stream;
use turso::Connection;

use crate::{
    api::homewizard::MeterMeasurement,
    core::interval::Interval,
    prelude::{instrument, *},
    quantity::{Quantity, energy::KilowattHours},
};

#[derive(Builder)]
pub struct BatteryLog {
    pub timestamp: DateTime<Local>,
    pub residual_energy: KilowattHours,
    pub meter: MeterMeasurement,
}

impl BatteryLog {
    #[instrument(
        skip_all,
        fields(
            residual = ?self.residual_energy,
            import = ?self.meter.import,
            export = ?self.meter.export,
        ),
    )]
    pub async fn insert_into(&self, connection: &Connection) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO battery_logs (
                timestamp_millis,
                residual_energy_kwh,
                import_kwh,
                export_kwh
            ) VALUES (?1, ?2, ?3, ?4)
        ";

        info!("inserting the battery log…");
        connection
            .prepare_cached(SQL)
            .await?
            .execute((
                self.timestamp.timestamp_millis(),
                self.residual_energy,
                self.meter.import,
                self.meter.export,
            ))
            .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn select_from(
        connection: &Connection,
        interval: Interval,
    ) -> Result<impl Stream<Item = Result<Self>>> {
        // language=sqlite
        const SQL: &str = r"
            SELECT
                timestamp_millis,   -- 0
                import_kwh,         -- 1
                export_kwh,         -- 2
                residual_energy_kwh -- 3
            FROM battery_logs
            WHERE timestamp_millis >= ?1 AND timestamp_millis < ?2
            ORDER BY timestamp_millis
        ";

        info!("querying battery logs…");
        let mut rows = connection
            .prepare_cached(SQL)
            .await?
            .query((interval.start.timestamp_millis(), interval.end.timestamp_millis()))
            .await
            .context("failed to query rows")?;
        let stream = try_stream! {
            while let Some(row) = rows.next().await.context("failed to fetch next row")? {
                yield Self::builder()
                    .timestamp(Local.timestamp_millis_opt(row.get(0)?).unwrap())
                    .meter(
                        MeterMeasurement::builder()
                            .import(Quantity(row.get(1)?))
                            .export(Quantity(row.get(2)?))
                        .build()
                    )
                    .residual_energy(Quantity(row.get(3)?))
                    .build()
            }
        };
        Ok(stream)
    }
}
