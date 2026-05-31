#![allow(clippy::wildcard_imports)]

use crate::quantity::{currency::*, energy::*, power::*, price::*, time::*};

macro_rules! mul {
    ($lhs:path, $rhs:path, $output:path) => {
        impl ::std::ops::Mul<$rhs> for $lhs {
            type Output = $output;

            fn mul(self, rhs: $rhs) -> Self::Output {
                <$output>::new(self.0 * rhs.0)
            }
        }

        impl ::std::ops::Mul<$lhs> for $rhs {
            type Output = $output;

            fn mul(self, lhs: $lhs) -> Self::Output {
                <$output>::new(lhs.0 * self.0)
            }
        }
    };
}

mul!(KilowattHourPrice, WattHours, Mills);
mul!(Watts, Hours, WattHours);
