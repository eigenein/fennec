use std::ops::{Div, Mul};

use chrono::TimeDelta;

use crate::quantity::{energy::KilowattHours, power::Kilowatts};

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, serde::Deserialize)]
pub struct BatteryStateAttributes<T> {
    #[serde(rename = "custom_battery_residual_energy")]
    pub residual_energy: T,

    #[serde(rename = "custom_battery_energy_import")]
    pub total_import: T,

    #[serde(rename = "custom_battery_energy_export")]
    pub total_export: T,
}

impl Div<TimeDelta> for BatteryStateAttributes<KilowattHours> {
    type Output = BatteryStateAttributes<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            residual_energy: self.residual_energy / rhs,
            total_import: self.total_import / rhs,
            total_export: self.total_export / rhs,
        }
    }
}

impl Mul<TimeDelta> for BatteryStateAttributes<Kilowatts> {
    type Output = BatteryStateAttributes<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            residual_energy: self.residual_energy * rhs,
            total_import: self.total_import * rhs,
            total_export: self.total_export * rhs,
        }
    }
}
