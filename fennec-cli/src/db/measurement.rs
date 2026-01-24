use bon::Builder;
use chrono::{DateTime, Local};

use crate::{api::homewizard::MeterMeasurement, quantity::energy::KilowattHours};

#[derive(Builder)]
#[must_use]
pub struct Measurement {
    pub timestamp: DateTime<Local>,
    pub total: MeterMeasurement,
    pub battery: MeterMeasurement,
    pub residual_energy: KilowattHours,
}
