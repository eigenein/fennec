use std::ops::{Add, Div, Index, Mul};

use derive_more::AddAssign;

use crate::{prelude::*, quantity::Zero};

#[must_use]
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<W, V> {
    pub weight: W,
    pub value: V,
}

impl<W, V> Integrator<W, V> {
    pub const fn new() -> Self
    where
        W: Zero,
        V: Zero,
    {
        Self { weight: W::ZERO, value: V::ZERO }
    }

    pub fn trapezoid<D>(weight: W, lhs: D, rhs: D) -> Self
    where
        D: Add<Output = D> + Div<f64, Output = D> + Mul<W, Output = V>,
        W: Clone,
    {
        Self { weight: weight.clone(), value: (lhs + rhs) / 2.0 * weight }
    }

    pub fn average(self) -> Option<<V as Div<W>>::Output>
    where
        V: Div<W>,
        W: Zero + PartialEq,
    {
        if self.weight == W::ZERO { None } else { Some(self.value / self.weight) }
    }
}

#[must_use]
pub struct BucketIntegrator<W, T> {
    pub total: Integrator<W, T>,
    pub buckets: Vec<Integrator<W, T>>,
}

impl<W, T> BucketIntegrator<W, T> {
    pub fn new(max_bucket_index: usize) -> Self
    where
        T: Zero,
        W: Zero,
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

impl<W, V> TryFrom<BucketIntegrator<W, V>> for BucketAverage<<V as Div<W>>::Output>
where
    V: Div<W>,
    W: Zero + PartialEq,
{
    type Error = Error;

    fn try_from(integrator: BucketIntegrator<W, V>) -> Result<Self> {
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
