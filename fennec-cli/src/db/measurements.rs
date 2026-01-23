use async_stream::try_stream;
use bon::Builder;
use chrono::{DateTime, Local, TimeZone};
use futures_core::stream::Stream;
use turso::Connection;

use crate::{
    api::homewizard::MeterMeasurement,
    core::interval::Interval,
    prelude::*,
    quantity::{Quantity, energy::KilowattHours},
};

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
                measurement.timestamp.timestamp_millis(),
                measurement.total.import,
                measurement.total.export,
                measurement.battery.import,
                measurement.battery.export,
                measurement.residual_energy,
            ))
            .await?;
        Ok(())
    }

    pub async fn select(
        &self,
        interval: Interval,
    ) -> Result<impl Stream<Item = Result<Measurement>>> {
        // language=sqlite
        const SQL: &str = r"
            SELECT
                timestamp_millis,   -- 0
                total_import_kwh,   -- 1
                total_export_kwh,   -- 2
                battery_import_kwh, -- 3
                battery_export_kwh, -- 4
                battery_energy_kwh  -- 5
            FROM measurements
            WHERE timestamp_millis >= ?1 AND timestamp_millis < ?2
            ORDER BY timestamp_millis
        ";

        info!("querying measurement history…");
        let mut rows = self
            .0
            .prepare_cached(SQL)
            .await?
            .query((interval.start.timestamp_millis(), interval.end.timestamp_millis()))
            .await
            .context("failed to query rows")?;
        let stream = try_stream! {
            while let Some(row) = rows.next().await.context("failed to fetch next row")? {
                yield Measurement::builder()
                    .timestamp(Local.timestamp_millis_opt(row.get(0)?).unwrap())
                    .total(
                        MeterMeasurement::builder()
                            .import(Quantity(row.get(1)?))
                            .export(Quantity(row.get(2)?))
                        .build()
                    )
                    .battery(
                        MeterMeasurement::builder()
                            .import(Quantity(row.get(3)?))
                            .export(Quantity(row.get(4)?))
                        .build()
                    )
                    .residual_energy(Quantity(row.get(5)?))
                    .build()
            }
        };
        Ok(stream)
    }
}

#[derive(Builder)]
pub struct Measurement {
    pub timestamp: DateTime<Local>,
    pub total: MeterMeasurement,
    pub battery: MeterMeasurement,
    pub residual_energy: KilowattHours,
}
