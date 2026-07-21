use std::{
    iter::Sum,
    ops::{Add, AddAssign, Mul, Sub},
    range::Range,
};

use musli::{Decode, Encode};

use crate::{
    math::{sinc, smoothing::Exponential},
    prelude::instrument,
    quantity::{
        Zero,
        angle::{Harmonic, Radians},
    },
};

#[must_use]
#[derive(Clone, Encode, Decode)]
pub struct ExponentialMovingDecomposition<V> {
    /// Global average energy balance (constant term of the Fourier decomposition).
    #[musli(Binary, name = 1)]
    mean: Exponential<V>,

    /// Energy balance harmonics (c₁ and so on).
    #[musli(Binary, name = 2)]
    harmonics: Vec<Exponential<Harmonic<V>>>,
}

impl<V: Clone + Zero> Default for ExponentialMovingDecomposition<V> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<V: Zero> ExponentialMovingDecomposition<V> {
    const DEFAULT_HARMONIC: Exponential<Harmonic<V>> = Exponential(Zero::ZERO);
}

impl<V> ExponentialMovingDecomposition<V> {
    pub fn new(n_harmonics: usize) -> Self
    where
        V: Clone + Zero,
    {
        Self { mean: Exponential(Zero::ZERO), harmonics: vec![Self::DEFAULT_HARMONIC; n_harmonics] }
    }

    #[must_use]
    pub const fn mean(&self) -> V
    where
        V: Copy,
    {
        self.mean.0
    }

    /// Adjust the number of harmonics.
    ///
    /// New harmonics are initialized with zeroes, extra harmonics get removed.
    pub fn resize(&mut self, n_harmonics: usize)
    where
        V: Clone + Zero,
    {
        self.harmonics.resize(n_harmonics, Self::DEFAULT_HARMONIC);
    }

    /// Calculate the deviation from the average at the given phase of the period.
    ///
    /// For example, 13:00 in daily cycle is 13π/12.
    #[must_use]
    pub fn deviation_at(&self, base_phase: Radians) -> V
    where
        V: Copy + Add<Output = V> + Mul<f64, Output = V> + Sum + Zero,
    {
        (1..)
            .map(f64::from)
            .zip(self.harmonics.iter())
            .map(|(mode_index, harmonic)| {
                harmonic.0.dot(Harmonic::from_phase(base_phase * mode_index))
            })
            .fold(Zero::ZERO, |sum, item| sum + item)
    }

    /// Calculate the mean deviation over the phase interval.
    ///
    /// Note that the interval must be unwrapped.
    /// For example, 23:00-01:00 in daily cycle should be represented as 23π/12..25π/12.
    #[must_use]
    pub fn mean_deviation_over(&self, interval: Range<Radians>) -> V
    where
        V: Copy + Zero + Add<Output = V> + Mul<f64, Output = V>,
    {
        assert!(interval.start < interval.end);
        let phase_radius = (interval.end - interval.start) / 2.0;
        let middle_phase = interval.start + phase_radius;
        (1..)
            .map(f64::from)
            .zip(&self.harmonics)
            .map(|(mode_index, harmonic)| {
                let weight = sinc(mode_index * phase_radius);
                harmonic.0.dot(Harmonic::from_phase(middle_phase * mode_index)) * weight
            })
            .fold(Zero::ZERO, |sum: V, weighted_value: V| sum + weighted_value)
    }

    /// Update the decomposition with an online value at the given phase.
    #[instrument(skip_all)]
    pub fn update(&mut self, value: V, base_phase: Radians, smoothing_factor: f64)
    where
        V: Copy + AddAssign + Sub<Output = V> + Mul<f64, Output = V>,
    {
        // Calculate the deviation before the mean update eats the signal:
        let deviation = value - self.mean.0;

        self.mean.update(value, smoothing_factor);

        for (mode_index, harmonic) in (1..).map(f64::from).zip(self.harmonics.iter_mut()) {
            let basis = Harmonic::from_phase(base_phase * mode_index);
            let target = Harmonic {
                // Multiplication by 2 comes from the scale factor:
                // <https://en.wikipedia.org/wiki/Fourier_series#Analysis>.
                cosine: deviation * (2.0 * basis.cosine),
                sine: deviation * (2.0 * basis.sine),
            };
            harmonic.update(target, smoothing_factor);
        }
    }
}
