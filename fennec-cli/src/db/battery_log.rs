use bon::Builder;
use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::{
    ClientSession,
    Collection,
    SessionCursor,
    options::{TimeseriesGranularity, TimeseriesOptions},
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    api::homewizard::MeterMeasurement,
    core::interval::Interval,
    db::SessionDb,
    prelude::*,
    quantity::energy::KilowattHours,
};

#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct BatteryLog {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "residualEnergyKilowattHours")]
    #[builder(into)]
    pub residual_energy: KilowattHours,

    #[serde(flatten)]
    pub meter: MeterMeasurement,
}

pub struct BatteryLogs<'session> {
    pub(super) collection: Collection<BatteryLog>,
    pub(super) session: &'session mut ClientSession,
}

impl<'session> BatteryLogs<'session> {
    pub(super) const COLLECTION_NAME: &'static str = "batteryLogs";

    #[instrument(skip_all)]
    pub(super) async fn initialize_on(db: &mut SessionDb) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(TimeseriesGranularity::Minutes)
            .build();
        db.create_timeseries(Self::COLLECTION_NAME, options).await
    }

    #[instrument(skip_all)]
    pub async fn insert(&mut self, log: &BatteryLog) -> Result {
        info!(
            residual = ?log.residual_energy,
            import = ?log.meter.import,
            export = ?log.meter.export,
            "inserting the battery log…",
        );
        self.collection
            .insert_one(log)
            .session(&mut *self.session)
            .await
            .context("failed to insert the battery log")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn find(&'session mut self, interval: Interval) -> Result<SessionCursor<BatteryLog>> {
        info!(?interval, "querying battery logs…");
        self.collection
            .find(doc! { "timestamp": { "$gte": interval.start, "$lt": interval.end } })
            .sort(doc! { "timestamp": -1 })
            .session(&mut *self.session)
            .await
            .context("failed to query the battery logs")
    }
}
