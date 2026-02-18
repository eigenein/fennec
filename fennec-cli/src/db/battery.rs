use bon::Builder;
use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{api::homewizard::EnergyMetrics, db, quantity::energy::KilowattHours};

/// Battery energy meter entry.
#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "residualEnergyKilowattHours")]
    #[builder(into)]
    pub residual_energy: KilowattHours,

    #[serde(flatten)]
    pub metrics: EnergyMetrics,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &'static str = "batteryLogs";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Minutes;
}
