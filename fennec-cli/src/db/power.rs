use bon::Builder;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{db, quantity::power::Watts};

/// Net power balance measurement.
#[serde_as]
#[derive(derive_more::Debug, Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    #[debug(skip)]
    pub timestamp: DateTime<Utc>,

    /// Net power balance.
    ///
    /// Positive value means insufficient PV power, negative value means excess PV power.
    ///
    /// Battery charging and discharging does not affect this value – exactly because our primary goal
    /// is to steer the battery on the basis of the net deficit.
    #[serde(rename = "netWatts")]
    pub net: Watts,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &str = "powerMeasurements";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Seconds;
}
