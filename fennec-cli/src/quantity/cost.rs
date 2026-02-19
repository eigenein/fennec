use std::ops::Div;

use crate::quantity::{energy::KilowattHours, rate::KilowattHourRate};

quantity!(Cost, via: f64, suffix: "€", precision: 3);

impl Cost {
    pub const ONE_CENT: Self = Self(0.01);
}

impl Div<KilowattHours> for Cost {
    type Output = KilowattHourRate;

    fn div(self, rhs: KilowattHours) -> Self::Output {
        KilowattHourRate(self.0 / rhs.0)
    }
}
