use std::{
    f64::consts::LN_2,
    mem::replace,
    ops::{AddAssign, Mul, Sub},
    time::Duration,
};

use chrono::{DateTime, Local, TimeDelta};
use musli::{Decode, Encode};

/// Raw [exponential moving average][1] with explicit smoothing factor per update.
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_smoothing
#[must_use]
#[derive(Encode, Decode)]
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
    pub const fn get(&self) -> &V {
        &self.0
    }

    pub fn get_decayed(&self, decay_factor: DecayFactor) -> V
    where
        V: Clone + Mul<f64, Output = V>,
    {
        self.0.clone() * decay_factor.0
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
#[derive(Copy, Clone)]
pub struct SmoothingFactor(f64);

/// Exponential [decay factor][1].
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_decay
#[must_use]
#[derive(Copy, Clone)]
pub struct DecayFactor(f64);

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
    pub const fn new(half_life: Duration) -> Self {
        Self(LN_2 / half_life.as_secs_f64())
    }

    /// Calculate the smoothing factor from the elapsed time.
    ///
    /// Algebraically, this is equivalent to one minus [`Self::decay_factor`], but more stable.
    pub fn smoothing_factor(self, elapsed: TimeDelta) -> SmoothingFactor {
        SmoothingFactor(-(-self.decay(elapsed)).exp_m1())
    }

    /// Calculate the decay factor.
    ///
    /// Algebraically, this is equivalent to one minus [`Self::smoothing_factor`], but more stable.
    pub fn decay_factor(self, elapsed: TimeDelta) -> DecayFactor {
        DecayFactor((-self.decay(elapsed)).exp())
    }

    /// λΔt measured in [nepers][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Neper
    fn decay(self, elapsed: TimeDelta) -> f64 {
        assert!(elapsed >= TimeDelta::zero(), "elapsed time must be non-negative");
        elapsed.as_seconds_f64() * self.0
    }
}

/// Exponential moving average with automatic temporal smoothing.
#[must_use]
#[derive(Encode, Decode)]
pub struct Clocked<V> {
    #[musli(Binary, name = 1)]
    smoother: Exponential<V>,

    #[musli(Binary, name = 2)]
    #[musli(with = crate::ops::musli::chrono)]
    last_updated_at: DateTime<Local>,
}

impl<V> Clocked<V> {
    pub const fn new(initial_value: V, initialized_at: DateTime<Local>) -> Self {
        Self { smoother: Exponential::new(initial_value), last_updated_at: initialized_at }
    }

    /// Get the current smoothed value.
    pub const fn get(&self) -> &V {
        self.smoother.get()
    }

    /// Get the current smoothed value decayed to the specified moment in time.
    pub fn get_decayed(&self, at: DateTime<Local>, decay: HalfLife) -> V
    where
        V: Clone + Mul<f64, Output = V>,
    {
        self.smoother.get_decayed(decay.decay_factor(at - self.last_updated_at))
    }

    /// Update the moving average according to the elapsed time and decay parameter.
    pub fn update(&mut self, value: V, at: DateTime<Local>, decay: HalfLife)
    where
        V: Clone + AddAssign + Sub<Output = V> + Mul<f64, Output = V>,
    {
        let elapsed = at - replace(&mut self.last_updated_at, at);
        self.smoother.update(value, decay.smoothing_factor(elapsed));
    }
}
