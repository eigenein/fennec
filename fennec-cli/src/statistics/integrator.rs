use std::ops::Div;

use chrono::TimeDelta;
use derive_more::AddAssign;

use crate::quantity::energy::KilowattHours;

/// Value accumulator over time.
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub time_delta: TimeDelta,
    pub value: T,
}

impl Default for Integrator<KilowattHours> {
    fn default() -> Self {
        Self { time_delta: TimeDelta::zero(), value: KilowattHours::ZERO }
    }
}

impl<T> Integrator<T> {
    pub fn average(self) -> Option<<T as Div<TimeDelta>>::Output>
    where
        T: Div<TimeDelta>,
    {
        if self.time_delta.is_zero() { None } else { Some(self.value / self.time_delta) }
    }
}
