use std::ops::{Add, Mul};

use derive_more::{Add, AddAssign, Sub};
use musli::{Decode, Encode};

use crate::quantity::{Zero, angle::Radians};

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

impl Harmonic<f64> {
    /// Construct harmonic from the phase.
    pub fn from_phase(phase: Radians) -> Self {
        Self { cosine: phase.0.cos(), sine: phase.0.sin() }
    }
}

impl<T> Harmonic<T> {
    pub fn dot<S>(self, other: Harmonic<S>) -> <<T as Mul<S>>::Output as Add>::Output
    where
        T: Mul<S>,
        <T as Mul<S>>::Output: Add,
    {
        self.cosine * other.cosine + self.sine * other.sine
    }
}
