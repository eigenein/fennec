use std::ops::{Div, Mul};

use chrono::{DateTime, Local, TimeDelta};

use crate::{
    api::home_assistant::{battery::BatteryStateAttributes, history::State},
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Copy, Clone, derive_more::Add, derive_more::Sub, serde::Serialize)]
pub struct EnergyState<T> {
    /// Net household energy usage excluding the energy systems.
    pub total_energy_usage: T,

    pub battery: BatteryStateAttributes<T>,
}

impl<V: From<f64>> From<State<BatteryStateAttributes<V>>> for (DateTime<Local>, EnergyState<V>) {
    /// Unpack the state for collection into a series.
    fn from(state: State<BatteryStateAttributes<V>>) -> Self {
        (
            state.last_changed_at,
            EnergyState { total_energy_usage: state.value.into(), battery: state.attributes },
        )
    }
}

impl Div<TimeDelta> for EnergyState<KilowattHours> {
    type Output = EnergyState<Kilowatts>;

    fn div(self, rhs: TimeDelta) -> Self::Output {
        EnergyState {
            total_energy_usage: self.total_energy_usage / rhs,
            battery: self.battery / rhs,
        }
    }
}

impl Mul<TimeDelta> for EnergyState<Kilowatts> {
    type Output = EnergyState<KilowattHours>;

    fn mul(self, rhs: TimeDelta) -> Self::Output {
        EnergyState {
            total_energy_usage: self.total_energy_usage * rhs,
            battery: self.battery * rhs,
        }
    }
}
