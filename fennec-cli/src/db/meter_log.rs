use bon::Builder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::api::homewizard::EnergyMetrics;

/// Household energy meter log entry.
#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct MeterLog {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    #[serde(flatten)]
    pub metrics: EnergyMetrics,
}
