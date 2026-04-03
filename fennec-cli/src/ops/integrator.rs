use std::ops::{Add, Div, Index, Mul};

use derive_more::AddAssign;

use crate::{
    prelude::*,
    quantity::{Zero, time::Hours},
};

/// Value accumulator over time.
#[must_use]
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<T> {
    pub duration: Hours,
    pub value: T,
}

impl<T> Integrator<T> {
    pub const fn new() -> Self
    where
        T: Zero,
    {
        Self { duration: Hours::ZERO, value: T::ZERO }
    }

    pub fn trapezoid<V>(duration: Hours, lhs: V, rhs: V) -> Self
    where
        V: Add<Output = V> + Div<f64, Output = V> + Mul<Hours, Output = T>,
    {
        Self { duration, value: (lhs + rhs) / 2.0 * duration }
    }

    pub fn average(self) -> Option<<T as Div<Hours>>::Output>
    where
        T: Div<Hours>,
    {
        if self.duration == Hours::ZERO { None } else { Some(self.value / self.duration) }
    }
}

#[must_use]
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

#[must_use]
pub struct BucketAverage<T> {
    /// Global average across the samples.
    total: T,

    buckets: Vec<Option<T>>,
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

impl<T> BucketAverage<T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buckets.iter().map(|average| average.as_ref().unwrap_or(&self.total))
    }
}
