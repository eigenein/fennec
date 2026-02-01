use async_stream::try_stream;
use bon::Builder;
use bson::doc;
use chrono::{DateTime, Local, TimeZone};
use futures_core::Stream;
use mongodb::{
    Collection,
    Database,
    options::{TimeseriesGranularity, TimeseriesOptions},
};
use serde::{Deserialize, Serialize};
use turso::Connection;

use crate::{
    api::homewizard::MeterMeasurement,
    core::interval::Interval,
    db::timestamp::serialize_timestamp,
    prelude::{instrument, *},
    quantity::{Quantity, energy::KilowattHours},
};

#[expect(clippy::unsafe_derive_deserialize)]
#[derive(Serialize, Deserialize, Builder)]
pub struct BatteryLog {
    #[serde(rename = "timestamp", serialize_with = "serialize_timestamp")]
    pub timestamp: DateTime<Local>,

    #[serde(rename = "residualEnergyKilowattHours")]
    pub residual_energy: KilowattHours,

    #[serde(flatten)]
    pub meter: MeterMeasurement,
}

impl BatteryLog {
    #[deprecated]
    #[instrument(skip_all)]
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

        info!(
            residual = ?self.residual_energy,
            import = ?self.meter.import,
            export = ?self.meter.export,
            "inserting the battery log…",
        );
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

    #[deprecated]
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

        info!(?interval, "querying battery logs…");
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

pub struct BatteryLogs(pub(super) Collection<BatteryLog>);

impl BatteryLogs {
    pub(super) const COLLECTION_NAME: &str = "batteryLogs";

    #[instrument(skip_all)]
    pub(super) async fn initialize_on(db: &Database) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(TimeseriesGranularity::Minutes)
            .build();
        if let Err(error) = db
            .create_collection(Self::COLLECTION_NAME)
            .timeseries(options)
            .await
            .context("failed to create the battery log collection")
        {
            // FIXME
            warn!("{error:#}");
        }
        db.run_command(doc! {
            "collMod": Self::COLLECTION_NAME,
            "expireAfterSeconds": 365 * 24 * 60 * 60, // FIXME: make configurable.
        })
        .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn insert(&self, log: &BatteryLog) -> Result {
        self.0.insert_one(log).await?;
        Ok(())
    }
}
