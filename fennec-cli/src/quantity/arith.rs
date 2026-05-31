//! Arithmetic for the quantities.

#![allow(clippy::wildcard_imports)]

use std::ops::Mul;

use crate::quantity::{
    Quantity,
    currency::*,
    energy::*,
    power::*,
    price::*,
    ratios::{BasisPoints, Percentage},
    time::*,
};

macro_rules! mul {
    ($lhs:path, $rhs:path, $output:path) => {
        impl Mul<$rhs> for $lhs {
            type Output = $output;

            fn mul(self, rhs: $rhs) -> Self::Output {
                <$output>::new(self.0 * rhs.0)
            }
        }

        impl Mul<$lhs> for $rhs {
            type Output = $output;

            fn mul(self, lhs: $lhs) -> Self::Output {
                <$output>::new(lhs.0 * self.0)
            }
        }
    };
}

mul!(KilowattHourPrice, WattHours, Mills);
mul!(Watts, Hours, WattHours);

/// Specialized implementation for [`Percentage`].
impl Mul<Self> for Percentage {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        Quantity(u16::from(self.0) * u16::from(rhs.0))
    }
}
