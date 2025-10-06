use std::ops::{Div, Mul, Sub};

use chrono::TimeDelta;

use crate::{
    api::home_assistant::history::State,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

impl Sub<Self> for State<KilowattHours, BatteryStateAttributes<KilowattHours>> {
    type Output = BatteryDifferentials<KilowattHours>;

    fn sub(self, rhs: Self) -> Self::Output {
        BatteryDifferentials {
            residual_energy: rhs.value - self.value,
            attributes: rhs.attributes - self.attributes,
        }
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, serde::Serialize, serde::Deserialize)]
pub struct BatteryStateAttributes<T> {
    #[serde(alias = "custom_battery_energy_import")]
    pub total_import: T,

    #[serde(alias = "custom_battery_energy_export")]
    pub total_export: T,
}

impl Div<TimeDelta> for BatteryStateAttributes<KilowattHours> {
    type Output = BatteryStateAttributes<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            total_import: self.total_import / rhs,
            total_export: self.total_export / rhs,
        }
    }
}

impl Mul<TimeDelta> for BatteryStateAttributes<Kilowatts> {
    type Output = BatteryStateAttributes<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        BatteryStateAttributes {
            total_import: self.total_import * rhs,
            total_export: self.total_export * rhs,
        }
    }
}

#[must_use]
#[derive(Copy, Clone, serde::Serialize)]
pub struct BatteryDifferentials<T> {
    pub residual_energy: T,
    pub attributes: BatteryStateAttributes<T>,
}

impl Div<TimeDelta> for BatteryDifferentials<KilowattHours> {
    type Output = BatteryDifferentials<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        BatteryDifferentials {
            residual_energy: self.residual_energy / rhs,
            attributes: self.attributes / rhs,
        }
    }
}
