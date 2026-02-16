use bon::Builder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{db::log::TimeSeries, quantity::energy::KilowattHours};

/// Household energy meter log entry.
#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct LogEntry {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "netKilowattHours")]
    pub net: KilowattHours,
}

impl TimeSeries for LogEntry {
    const COLLECTION_NAME: &str = "consumptionLogs";
}
