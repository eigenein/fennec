use std::{
    f64::consts::LN_2,
    ops::{AddAssign, Mul, Sub},
};

use chrono::{DateTime, Local, TimeDelta};

/// Raw [exponential moving average][1] with explicit smoothing factor per update.
///
/// [1]: https://en.wikipedia.org/wiki/Exponential_smoothing
#[must_use]
pub struct Exponential<V>(
    /// Smoothed value.
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

    pub fn update<F>(&mut self, value: V, factor: F)
    where
        V: Clone + AddAssign + Sub<Output = V> + Mul<F, Output = V>,
    {
        self.0 += (value - self.0.clone()) * factor;
    }
}

/// Exponential moving average with explicit temporal smoothing factor per update.
#[must_use]
pub struct HalfLife<V> {
    /// Inner raw exponential smoother.
    smoother: Exponential<V>,

    /// Lambda of the exponential decay, [`LN_2`] divided by the half-time – in [nepers][1] per second, Nps⁻¹.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Neper
    decay_rate_per_sec: f64,
}

impl<V> HalfLife<V> {
    pub fn new(initial_value: V, half_life: TimeDelta) -> Self {
        Self {
            smoother: Exponential::new(initial_value),
            decay_rate_per_sec: LN_2 / half_life.as_seconds_f64(),
        }
    }

    /// Get the smoothed value.
    pub const fn get(&self) -> &V {
        &self.smoother.0
    }

    pub fn update(&mut self, value: V, elapsed: TimeDelta)
    where
        V: Clone + AddAssign + Sub<Output = V> + Mul<f64, Output = V>,
    {
        self.smoother.update(value, self.smoothing_factor(elapsed));
    }

    /// Calculate the smoothing factor from the elapsed time.
    fn smoothing_factor(&self, elapsed: TimeDelta) -> f64 {
        assert!(elapsed >= TimeDelta::zero(), "elapsed time must be non-negative");
        let decay = elapsed.as_seconds_f64() * self.decay_rate_per_sec;
        -(-decay).exp_m1()
    }
}

/// Exponential moving average with automatic temporal smoothing.
#[must_use]
pub struct Clocked<V> {
    smoother: HalfLife<V>,
    last_updated_at: DateTime<Local>,
}

impl<V> Clocked<V> {
    pub fn new(initial_value: V, initialized_at: DateTime<Local>, half_life: TimeDelta) -> Self {
        Self { smoother: HalfLife::new(initial_value, half_life), last_updated_at: initialized_at }
    }

    /// Get the smoothed value.
    pub const fn get(&self) -> &V {
        self.smoother.get()
    }

    pub fn update(&mut self, value: V, at: DateTime<Local>)
    where
        V: Clone + AddAssign + Sub<Output = V> + Mul<f64, Output = V>,
    {
        self.smoother.update(value, at - self.last_updated_at);
        self.last_updated_at = at;
    }
}
