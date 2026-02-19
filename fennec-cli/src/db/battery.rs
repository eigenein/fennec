use bon::Builder;
use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    db,
    quantity::energy::{KilowattHours, WattHours},
};

/// Battery energy meter entry.
#[serde_as]
#[derive(derive_more::Debug, Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    #[debug(skip)]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "residualEnergyKilowattHours")]
    #[builder(into)]
    pub legacy_residual_energy: KilowattHours,

    #[serde(rename = "residualEnergyWattHours")]
    #[builder(into)]
    pub residual_energy: Option<WattHours>,

    #[serde(rename = "importKilowattHours")]
    pub legacy_import: KilowattHours,

    #[serde(rename = "importWattHours")]
    #[builder(into)]
    pub import: Option<WattHours>,

    #[serde(rename = "exportKilowattHours")]
    pub legacy_export: KilowattHours,

    #[serde(rename = "exportWattHours")]
    #[builder(into)]
    pub export: Option<WattHours>,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &'static str = "batteryLogs";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Minutes;
}
