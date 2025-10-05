use std::ops::{Div, Mul};

use chrono::TimeDelta;

use crate::{
    api::home_assistant::history::State,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, serde::Serialize)]
pub struct BatteryState<T> {
    pub residual_energy: T,
    pub attributes: BatteryStateAttributes<T>,
}

impl<T> From<State<T, BatteryStateAttributes<T>>> for BatteryState<T> {
    fn from(state: State<T, BatteryStateAttributes<T>>) -> Self {
        Self { residual_energy: state.value, attributes: state.attributes }
    }
}

impl Div<TimeDelta> for BatteryState<KilowattHours> {
    type Output = BatteryState<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        BatteryState {
            residual_energy: self.residual_energy / rhs,
            attributes: self.attributes / rhs,
        }
    }
}

impl Mul<TimeDelta> for BatteryState<Kilowatts> {
    type Output = BatteryState<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        BatteryState {
            residual_energy: self.residual_energy * rhs,
            attributes: self.attributes * rhs,
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
