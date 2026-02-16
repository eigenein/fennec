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

    /// Total lifetime PV yield.
    ///
    /// It will become required in the future.
    #[serde(rename = "pvKilowattHours")]
    pub pv_yield: Option<KilowattHours>,

    /// Total lifetime net contribution from the grid and battery.
    ///
    /// Hint: if you add the PV yield, you will get the total consumption.
    #[serde(rename = "netKilowattHours")]
    pub pv_deficit: KilowattHours,
}

impl TimeSeries for LogEntry {
    const COLLECTION_NAME: &str = "consumptionLogs";
}
