use bon::Builder;
use chrono::{DateTime, Local};

use crate::quantity::energy::MilliwattHours;

#[derive(Builder)]
pub struct ResidualEnergyTransition {
    pub timestamp: DateTime<Local>,
    pub energy: MilliwattHours,
}
