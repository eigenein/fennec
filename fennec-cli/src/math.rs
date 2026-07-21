use std::f64::consts::PI;

pub mod smoothing;

/// Normalized [sinc](https://en.wikipedia.org/wiki/Sinc_function) function: sin(πx)÷(πx).
pub fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let pi_x = PI * x;
        pi_x.sin() / pi_x
    }
}
