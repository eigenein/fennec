use std::collections::BTreeMap;

use chrono::{DateTime, Local, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::quantity::rate::KilowattHourRate;

#[derive(Default, Serialize, Deserialize)]
pub struct ProviderStatistics {
    /// Note: key refers to the interval start time.
    pub history: BTreeMap<DateTime<Local>, KilowattHourRate>,

    #[serde(default)]
    pub medians: BTreeMap<NaiveTime, KilowattHourRate>,
}
