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

    /// Area under the [trapezoid][1] with the base `weight` and the legs `lhs` and `rhs`.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Trapezoid
    pub fn trapezoid<D>(weight: W, lhs: D, rhs: D) -> Self
    where
        D: Add<Output = D> + Div<f64, Output = D> + Mul<W, Output = V>,
        W: Clone,
    {
        Self { weight: weight.clone(), value: (lhs + rhs) / 2.0 * weight }
    }

    /// Calculate [the mean of the integrated function][1]
    ///
    /// [1]: https://en.wikipedia.org/wiki/Mean_of_a_function
    pub fn mean(self) -> Option<<V as Div<W>>::Output>
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
pub struct BucketMean<T> {
    /// Global average across the samples.
    total: T,

    buckets: Vec<Option<T>>,
}

impl<W, V> TryFrom<BucketIntegrator<W, V>> for BucketMean<<V as Div<W>>::Output>
where
    V: Div<W>,
    W: Zero + PartialEq,
{
    type Error = Error;

    fn try_from(integrator: BucketIntegrator<W, V>) -> Result<Self> {
        Ok(Self {
            total: integrator.total.mean().context("no samples to calculate the total average")?,
            buckets: integrator.buckets.into_iter().map(Integrator::mean).collect(),
        })
    }
}

impl<T> Index<usize> for BucketMean<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.buckets[index].as_ref().unwrap_or(&self.total)
    }
}

impl<T> BucketMean<T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buckets.iter().map(|average| average.as_ref().unwrap_or(&self.total))
    }
}
