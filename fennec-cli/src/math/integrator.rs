use std::ops::{Add, Div, Mul};

use derive_more::AddAssign;

use crate::quantity::Zero;

#[must_use]
#[derive(Copy, Clone, AddAssign)]
pub struct Integrator<W, V> {
    pub weight: W,
    pub value: V,
}

impl<W, V> Integrator<W, V> {
    pub const fn new() -> Self
    where
        W: Zero,
        V: Zero,
    {
        Self { weight: W::ZERO, value: V::ZERO }
    }

    /// Area under the [trapezoid][1] with the base `weight` and the legs `lhs` and `rhs`.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Trapezoid
    pub fn trapezoid<D>(weight: W, lhs: D, rhs: D) -> Self
    where
        D: Add<Output = D> + Div<f64, Output = D> + Mul<W, Output = V>,
        W: Clone,
    {
        Self { weight: weight.clone(), value: (lhs + rhs) / 2.0 * weight }
    }

    /// Calculate [the mean of the integrated function][1]
    ///
    /// [1]: https://en.wikipedia.org/wiki/Mean_of_a_function
    pub fn mean(self) -> Option<<V as Div<W>>::Output>
    where
        V: Div<W>,
        W: Zero + PartialEq,
    {
        if self.weight == W::ZERO { None } else { Some(self.value / self.weight) }
    }
}
