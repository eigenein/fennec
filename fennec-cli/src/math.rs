use std::f64::consts::PI;

use crate::quantity::angle::Radians;

pub mod fourier;
pub mod smoothing;

/// Non-normalized [sinc](https://en.wikipedia.org/wiki/Sinc_function) function: sin(x)÷x.
#[must_use]
pub fn sinc(x: Radians) -> f64 {
    if x.0 == 0.0 { 1.0 } else { x.0.sin() / x.0 }
}

/// Normalized [sinc](https://en.wikipedia.org/wiki/Sinc_function) function: sin(πx)÷(πx).
#[must_use]
pub fn normalized_sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let pi_x = PI * x;
        pi_x.sin() / pi_x
    }
}
