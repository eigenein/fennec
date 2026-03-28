use std::ops::{Add, Div, Mul};

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

    pub fn trapezoid<V>(time_delta: Hours, lhs: V, rhs: V) -> Self
    where
        V: Add<Output = V> + Div<f64, Output = V> + Mul<Hours, Output = T>,
    {
        Self { time: time_delta, value: (lhs + rhs) / 2.0 * time_delta }
    }

    pub fn average(self) -> Option<<T as Div<Hours>>::Output>
    where
        T: Div<Hours>,
    {
        if self.time == Hours::ZERO { None } else { Some(self.value / self.time) }
    }
}
