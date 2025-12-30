use std::collections::HashMap;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::{core::provider::Provider, quantity::rate::KilowattHourRate};

#[derive(Default, Serialize, Deserialize)]
pub struct RateStatistics {
    pub of: HashMap<Provider, PerProviderRates>,
}

#[derive(Serialize, Deserialize)]
pub struct PerProviderRates {
    /// Note: key refers to the interval start time.
    pub history: HashMap<DateTime<Local>, KilowattHourRate>,
}
