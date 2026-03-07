use std::ops::Div;

use derive_more::AddAssign;

use crate::quantity::{Zero, time::Hours};

/// Value accumulator over time.
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub time: Hours,
    pub value: T,
}

impl<T> Integrator<T> {
    pub const fn new() -> Self
    where
        T: Zero,
    {
        Self { time: Hours::ZERO, value: T::ZERO }
    }

    pub fn average(self) -> Option<<T as Div<Hours>>::Output>
    where
        T: Div<Hours>,
    {
        if self.time == Hours::ZERO { None } else { Some(self.value / self.time) }
    }
}
