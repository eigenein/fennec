use std::ops::Div;

use derive_more::AddAssign;

use crate::quantity::time::Hours;

/// Value accumulator over time.
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub hours: Hours,
    pub value: T,
}

impl<T> Integrator<T> {
    pub const fn new(init: T) -> Self {
        Self { hours: Hours::zero(), value: init }
    }

    pub fn average(self) -> Option<<T as Div<Hours>>::Output>
    where
        T: Div<Hours>,
    {
        if self.hours.is_zero() { None } else { Some(self.value / self.hours) }
    }
}
