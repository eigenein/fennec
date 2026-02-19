use std::ops::Mul;

use crate::quantity::proportions::BasisPoints;

quantity!(Percent, u16, "%");

impl Percent {
    pub const fn to_proportion(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Mul<Self> for Percent {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        BasisPoints::from(self.0 * rhs.0)
    }
}
