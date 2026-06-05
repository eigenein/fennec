use std::ops::Mul;

use derive_more::{Add, AddAssign, Sub};
use musli::{Decode, Encode};

use crate::quantity::Zero;

/// As single [harmonic][1] from a [harmonic spectrum][2].
///
/// [1]: https://en.wikipedia.org/wiki/Harmonic
/// [2]: https://en.wikipedia.org/wiki/Harmonic_spectrum
#[derive(Copy, Clone, Add, AddAssign, Sub, Encode, Decode)]
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

impl<T: Mul<S>, S: Copy> Mul<S> for Harmonic<T> {
    type Output = Harmonic<<T as Mul<S>>::Output>;

    fn mul(self, rhs: S) -> Self::Output {
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
        T: Copy + Mul<f64, Output = T>,
    {
        let phase = base_phase * mode_index.into();

        // Multiplication by 2 comes from the scale factor: https://en.wikipedia.org/wiki/Fourier_series#Analysis.
        Self { cosine: signal * (2.0 * phase.cos()), sine: signal * (2.0 * phase.sin()) }
    }
}
