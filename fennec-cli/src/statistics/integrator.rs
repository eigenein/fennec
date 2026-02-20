use std::ops::Div;

use derive_more::AddAssign;

use crate::quantity::{Zero, time::Hours};

/// Value accumulator over time.
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub hours: Hours,
    pub value: T,
}

impl<T> Integrator<T> {
    pub const fn new(init: T) -> Self {
        Self { hours: Hours::ZERO, value: init }
    }

    pub fn average(self) -> Option<<T as Div<Hours>>::Output>
    where
        T: Div<Hours>,
    {
        if self.hours == Hours::ZERO { None } else { Some(self.value / self.hours) }
    }
}
