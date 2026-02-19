use std::ops::Mul;

quantity!(Percentage, u16, "%");
quantity!(BasisPoints, u16, "‱");

impl Percentage {
    pub const fn to_proportion(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Mul<Self> for Percentage {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        BasisPoints(self.0 * rhs.0)
    }
}
