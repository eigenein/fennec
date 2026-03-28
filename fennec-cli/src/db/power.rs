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

    /// Net power deficit on the main connection.
    ///
    /// Positive is net consumption, negative is net production.
    ///
    /// This is equal to «P1 net consumption plus battery net production»,
    /// as we only need to track and compensate the net deficit, hence:
    ///
    /// - battery charging or discharging has no effect on it;
    /// - PV production lowers it.
    #[serde(rename = "netWatts")]
    pub net_deficit: Watts,

    /// Active EPS power.
    ///
    /// We track it separately, because in all modes, the battery serves the demand on this output
    /// and competes for the inverter maximum power.
    #[serde(rename = "epsActivePower")]
    pub eps_active_power: Watts,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &str = "powerMeasurements";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Seconds;
}
