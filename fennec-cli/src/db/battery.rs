use bon::Builder;
use bson::doc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{api::homewizard::EnergyMetrics, db::log::Log, quantity::energy::KilowattHours};

/// Battery energy meter entry.
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
    pub metrics: EnergyMetrics,
}

impl Log for BatteryLog {
    const COLLECTION_NAME: &'static str = "batteryLogs";
}
