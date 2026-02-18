use bon::Builder;
use chrono::{DateTime, Timelike, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{db, quantity::energy::KilowattHours};

/// Household energy meter log entry.
#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    /// Total lifetime net contribution from the grid and battery.
    ///
    /// Hint: if you add the PV yield, you will get the total consumption.
    #[serde(rename = "netKilowattHours")]
    pub net_deficit: KilowattHours,
}

impl Measurement {
    /// Check whether the other log entry belongs to the same day and hour.
    #[must_use]
    pub fn same_hour_as(&self, other: &Self) -> bool {
        (self.timestamp.date_naive(), self.timestamp.hour())
            == (other.timestamp.date_naive(), other.timestamp.hour())
    }
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &str = "consumptionLogs";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Minutes;
}
