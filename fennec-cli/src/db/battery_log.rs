use bon::Builder;
use chrono::{DateTime, Local};

use crate::{api::homewizard::MeterMeasurement, quantity::energy::KilowattHours};

#[derive(Builder)]
pub struct BatteryLog {
    pub timestamp: DateTime<Local>,
    pub residual_energy: KilowattHours,
    pub meter_measurement: MeterMeasurement,
}
