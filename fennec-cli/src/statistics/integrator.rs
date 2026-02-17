use std::ops::Div;

use chrono::TimeDelta;
use derive_more::AddAssign;

/// Value accumulator over time.
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub time_delta: TimeDelta,
    pub value: T,
}

impl<T: Default> Default for Integrator<T> {
    fn default() -> Self {
        Self { time_delta: TimeDelta::zero(), value: T::default() }
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
