use std::ops::Mul;

quantity!(Percentage, via: u16, suffix: "%", precision: 1);
quantity!(BasisPoints, via: u16, suffix: "‱", precision: 0);

impl Percentage {
    /// Convert the percentage into `0.0..=1.0`.
    pub const fn to_ratio(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Mul<Self> for Percentage {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        BasisPoints(self.0 * rhs.0)
    }
}
