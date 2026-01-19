use std::collections::BTreeMap;

use chrono::{DateTime, Local, NaiveTime};
use quantities::rate::KilowattHourRate;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct ProviderStatistics {
    /// Note: key refers to the interval start time.
    pub history: BTreeMap<DateTime<Local>, KilowattHourRate>,

    #[serde(default)]
    pub medians: BTreeMap<NaiveTime, KilowattHourRate>,
}
