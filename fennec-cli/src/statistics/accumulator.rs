use chrono::TimeDelta;
use derive_more::AddAssign;

use crate::quantity::{energy::KilowattHours, power::Kilowatts};

#[derive(Copy, Clone, AddAssign)]
pub struct EnergyAccumulator {
    pub time_delta: TimeDelta,
    pub value: KilowattHours,
}

impl Default for EnergyAccumulator {
    fn default() -> Self {
        Self { time_delta: TimeDelta::zero(), value: KilowattHours::ZERO }
    }
}

impl EnergyAccumulator {
    pub fn average_power(self) -> Option<Kilowatts> {
        if self.time_delta.is_zero() { None } else { Some(self.value / self.time_delta) }
    }
}
