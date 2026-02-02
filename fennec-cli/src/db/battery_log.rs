use bon::Builder;
use bson::doc;
use chrono::{DateTime, Utc};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use mongodb::{
    Collection,
    options::{TimeseriesGranularity, TimeseriesOptions},
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    api::homewizard::MeterMeasurement,
    core::interval::Interval,
    db::Db,
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

pub struct BatteryLogs(Collection<BatteryLog>);

impl From<&Db> for BatteryLogs {
    fn from(db: &Db) -> Self {
        Self(db.0.collection(Self::COLLECTION_NAME))
    }
}

impl BatteryLogs {
    const COLLECTION_NAME: &'static str = "batteryLogs";

    #[instrument(skip_all)]
    pub(super) async fn initialize(db: &Db) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(TimeseriesGranularity::Minutes)
            .build();
        db.create_timeseries(Self::COLLECTION_NAME, options).await
    }

    #[instrument(skip_all)]
    pub async fn insert(&self, log: &BatteryLog) -> Result {
        info!(
            residual = ?log.residual_energy,
            import = ?log.meter.import,
            export = ?log.meter.export,
            "inserting the battery log…",
        );
        self.0.insert_one(log).await.context("failed to insert the battery log")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn find(
        &self,
        interval: Interval,
    ) -> Result<impl TryStream<Ok = BatteryLog, Error = Error>> {
        info!(?interval, "querying battery logs…");
        Ok(self
            .0
            .find(doc! { "timestamp": { "$gte": interval.start, "$lt": interval.end } })
            .sort(doc! { "timestamp": -1 })
            .await
            .context("failed to query the battery logs")?
            .map_err(Error::from))
    }
}
