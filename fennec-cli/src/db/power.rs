use bon::Builder;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{db, quantity::power::Watts};

/// Net power deficit measurement.
#[serde_as]
#[derive(Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    pub timestamp: DateTime<Utc>,

    /// Net power deficit.
    ///
    /// Positive value means insufficient PV power, negative value means excess PV power.
    ///
    /// Battery charging and discharging does not affect this value – exactly because our primary goal
    /// is to steer the battery on the basis of the net deficit.
    #[serde(rename = "deficitWatts")]
    pub deficit: Watts,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &str = "powerMeasurements";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Seconds;
}
