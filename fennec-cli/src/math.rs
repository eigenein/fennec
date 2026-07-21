use crate::quantity::angle::Radians;

pub mod fourier;
pub mod smoothing;

/// Non-normalized [sinc](https://en.wikipedia.org/wiki/Sinc_function) function: sin(x)÷x.
#[must_use]
pub fn sinc(x: Radians) -> f64 {
    if x.0 == 0.0 { 1.0 } else { x.0.sin() / x.0 }
}
