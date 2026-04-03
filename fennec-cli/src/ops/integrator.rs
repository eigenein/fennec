use std::ops::{Add, Div, Index, Mul};

use derive_more::AddAssign;

use crate::{
    prelude::*,
    quantity::{Zero, time::Hours},
};

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

pub struct BucketIntegrator<T> {
    pub total: Integrator<T>,
    pub buckets: Vec<Integrator<T>>,
}

impl<T> BucketIntegrator<T> {
    pub fn new(max_bucket_index: usize) -> Self
    where
        T: Zero,
    {
        Self {
            total: Integrator::new(),
            buckets: (0..=max_bucket_index).map(|_| Integrator::new()).collect(),
        }
    }
}

pub struct BucketAverage<T> {
    /// Global average across the samples.
    pub total: T,

    pub buckets: Vec<Option<T>>,
}

impl<T: Div<Hours>> TryFrom<BucketIntegrator<T>> for BucketAverage<<T as Div<Hours>>::Output> {
    type Error = Error;

    fn try_from(integrator: BucketIntegrator<T>) -> Result<Self> {
        Ok(Self {
            total: integrator
                .total
                .average()
                .context("no samples to calculate the total average")?,
            buckets: integrator.buckets.into_iter().map(Integrator::average).collect(),
        })
    }
}

impl<T> Index<usize> for BucketAverage<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.buckets[index].as_ref().unwrap_or(&self.total)
    }
}
