use std::ops::{Div, Mul};

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

impl<T: Mul<f64>> Mul<f64> for Harmonic<T> {
    type Output = Harmonic<<T as Mul<f64>>::Output>;

    fn mul(self, rhs: f64) -> Self::Output {
        Harmonic { cosine: self.cosine * rhs, sine: self.sine * rhs }
    }
}

impl<T: Div<f64>> Div<f64> for Harmonic<T> {
    type Output = Harmonic<<T as Div<f64>>::Output>;

    fn div(self, rhs: f64) -> Self::Output {
        Harmonic { cosine: self.cosine / rhs, sine: self.sine / rhs }
    }
}

impl<T> Harmonic<T> {
    /// Scale `signal` by the harmonic basis vector (cos φ, sin φ) at the given `phase`.
    pub fn scale(signal: T, phase: f64) -> Harmonic<<T as Mul<f64>>::Output>
    where
        T: Copy + Mul<f64>,
    {
        Harmonic { cosine: signal * phase.cos(), sine: signal * phase.sin() }
    }
}
