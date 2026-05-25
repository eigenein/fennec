use std::{
    f64::consts::TAU,
    ops::{Add, AddAssign, Mul, Range, Sub},
};

use derive_more::{AddAssign, Sub};
use musli::{Decode, Encode};

use crate::quantity::Zero;

#[derive(Clone, Encode, Decode)]
pub struct Decomposition<T> {
    /// Zero-frequency component.
    #[musli(Binary, name = 1)]
    pub mean: T,

    #[musli(Binary, name = 2)]
    harmonics: Vec<Harmonic<T>>,
}

impl<T: AddAssign> AddAssign for Decomposition<T> {
    fn add_assign(&mut self, rhs: Self) {
        assert_eq!(self.harmonics.len(), rhs.harmonics.len());
        self.mean += rhs.mean;
        for (lhs, rhs) in self.harmonics.iter_mut().zip(rhs.harmonics) {
            *lhs += rhs;
        }
    }
}

impl<T: Sub<Output = T>> Sub for Decomposition<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(self.harmonics.len(), rhs.harmonics.len());
        Self {
            mean: self.mean - rhs.mean,
            harmonics: self
                .harmonics
                .into_iter()
                .zip(rhs.harmonics)
                .map(|(lhs, rhs)| lhs - rhs)
                .collect(),
        }
    }
}

impl<T: Mul<f64, Output = T>> Mul<f64> for Decomposition<T> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            mean: self.mean * rhs,
            harmonics: self.harmonics.into_iter().map(|harmonic| harmonic * rhs).collect(),
        }
    }
}

impl<T> Decomposition<T> {
    /// Fourier decomposition with zeroed mean and harmonics.
    pub fn zero(n_harmonics: usize) -> Self
    where
        T: Copy + Zero,
    {
        Self { mean: T::ZERO, harmonics: vec![Harmonic::ZERO; n_harmonics] }
    }

    /// Calculate deviation from the mean at the given base phase.
    pub fn deviation_at(&self, base_phase: f64) -> T
    where
        T: Copy + Add<Output = T> + Mul<f64, Output = T> + Zero,
    {
        (1..)
            .zip(self.harmonics.iter())
            .map(|(mode_index, harmonic)| {
                let phase = base_phase * f64::from(mode_index);
                harmonic.cosine * phase.cos() + harmonic.sine * phase.sin()
            })
            .fold(T::ZERO, |sum, item| sum + item)
    }

    /// Calculate the mean deviation of the balance over the given interval,
    /// assuming the function is periodic over the unit interval.
    #[expect(clippy::float_cmp)]
    pub fn mean_deviation_over(&self, interval: Range<f64>) -> T
    where
        T: Copy + Zero + Add<Output = T> + Mul<f64, Output = T>,
    {
        assert_ne!(interval.start, interval.end);

        let length = interval.end - interval.start;
        (1..)
            .zip(self.harmonics.iter())
            .map(|(mode_index, harmonic)| {
                let angular_frequency = TAU * f64::from(mode_index);
                let cosine_mean = ((angular_frequency * interval.end).sin()
                    - (angular_frequency * interval.start).sin())
                    / angular_frequency
                    / length;
                let sine_mean = ((angular_frequency * interval.start).cos()
                    - (angular_frequency * interval.end).cos())
                    / angular_frequency
                    / length;
                harmonic.cosine * cosine_mean + harmonic.sine * sine_mean
            })
            .fold(T::ZERO, |sum, item| sum + item)
    }

    /// TODO: I'm honestly unsure how to name this operation. Claude, got ideas?
    pub fn project(&self, signal: T, base_phase: f64) -> Self
    where
        T: Copy + Sub<Output = T> + Mul<f64, Output = T>,
    {
        let deviation = signal - self.mean;

        Self {
            // The mean is going to tend to the signal:
            mean: signal,

            // The harmonics, on the other hand, tend to the projections onto the Fourier series:
            harmonics: (1..)
                .take(self.harmonics.len())
                .map(|mode_index| Harmonic::project(deviation, base_phase, mode_index))
                .collect(),
        }
    }
}

/// As single [harmonic][1] from a [harmonic spectrum][2].
///
/// [1]: https://en.wikipedia.org/wiki/Harmonic
/// [2]: https://en.wikipedia.org/wiki/Harmonic_spectrum
#[derive(Copy, Clone, AddAssign, Sub, Encode, Decode)]
pub struct Harmonic<T> {
    /// Fourier cosine coefficient.
    #[musli(Binary, name = 1)]
    pub cosine: T,

    /// Fourier sine coefficient.
    #[musli(Binary, name = 2)]
    pub sine: T,
}

impl<T: Zero> Zero for Harmonic<T> {
    const ZERO: Self = Self { cosine: T::ZERO, sine: T::ZERO };
}

impl<T: Mul<f64>> Mul<f64> for Harmonic<T> {
    type Output = Harmonic<<T as Mul<f64>>::Output>;

    fn mul(self, rhs: f64) -> Self::Output {
        Harmonic { cosine: self.cosine * rhs, sine: self.sine * rhs }
    }
}

impl<T> Harmonic<T> {
    /// Project the signal onto the harmonic.
    pub fn project(
        signal: T,
        base_phase: f64,
        mode_index: impl Into<f64>,
    ) -> Harmonic<<T as Mul<f64>>::Output>
    where
        T: Copy + Mul<f64>,
    {
        let phase = base_phase * mode_index.into();

        // Multiplication by 2 comes from the scale factor: https://en.wikipedia.org/wiki/Fourier_series#Analysis.
        Harmonic { cosine: signal * (2.0 * phase.cos()), sine: signal * (2.0 * phase.sin()) }
    }
}
