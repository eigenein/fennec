use std::{
    f64::consts::LN_2,
    ops::{AddAssign, Mul, Sub},
    time::Duration,
};

use chrono::TimeDelta;
use musli::{Decode, Encode};

/// Raw [exponential moving average][1] with explicit smoothing factor per update.
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_smoothing
#[must_use]
#[derive(Clone, Encode, Decode)]
pub struct Exponential<V>(
    /// Smoothed value.
    #[musli(name = 1)]
    V,
);

impl<V> Exponential<V> {
    pub const fn new(initial_value: V) -> Self {
        Self(initial_value)
    }

    /// Get the smoothed value.
    pub const fn value(&self) -> &V {
        &self.0
    }

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
pub struct HalfLife(
    /// Lambda of the exponential decay, [`LN_2`] divided by the half-time – in [nepers][1] per second, Nps⁻¹.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Neper
    f64,
);

impl HalfLife {
    pub fn new(half_life: impl Into<Duration>) -> Self {
        Self(LN_2 / half_life.into().as_secs_f64())
    }

    /// Calculate the smoothing factor from the elapsed time.
    ///
    /// Algebraically, this is equivalent to one minus [`Self::decay_factor`], but more stable.
    pub fn smoothing_factor(self, elapsed: TimeDelta) -> SmoothingFactor {
        SmoothingFactor(-(-self.decay(elapsed)).exp_m1())
    }

    /// λΔt measured in [nepers][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Neper
    fn decay(self, elapsed: TimeDelta) -> f64 {
        assert!(elapsed >= TimeDelta::zero(), "elapsed time must be non-negative");
        elapsed.as_seconds_f64() * self.0
    }
}
