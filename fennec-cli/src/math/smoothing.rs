use std::{
    f64::consts::LN_2,
    ops::{AddAssign, Div, Mul, Sub},
};

use musli::{Decode, Encode};

/// Raw [exponential moving average][1] with explicit smoothing factor per update.
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_smoothing
#[must_use]
#[derive(Clone, Encode, Decode)]
pub struct Exponential<V>(
    /// The underlying smoothed value.
    #[musli(name = 1)]
    pub V,
);

impl<V> Exponential<V> {
    /// Update the value.
    pub fn update(&mut self, value: V, smoothing_factor: SmoothingFactor)
    where
        V: Clone + AddAssign + Sub<Output = V> + Mul<f64, Output = V>,
    {
        self.0 += (value - self.0.clone()) * smoothing_factor.0;
    }
}

/// Exponential [smoothing factor][1].
///
/// Note: larger values of smoothing factor actually reduce the level of smoothing.
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_smoothing#:~:text=available.%20The%20term-,smoothing%20factor,-applied%20to
#[must_use]
#[derive(Copy, Clone, derive_more::Debug)]
#[debug("{_0}")]
pub struct SmoothingFactor(f64);

/// Half-life-parameterized exponential decay.
#[must_use]
#[derive(Copy, Clone)]
pub struct HalfLife<V>(pub V);

impl<V> HalfLife<V> {
    /// Calculate the smoothing factor from the quantity delta.
    pub fn smoothing_factor(self, delta: V) -> SmoothingFactor
    where
        V: Div<V, Output = f64>,
    {
        SmoothingFactor(-(-LN_2 * (delta / self.0)).exp_m1())
    }
}
