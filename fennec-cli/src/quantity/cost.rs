use std::{
    fmt::{Debug, Display, Formatter},
    ops::Div,
};

use crate::quantity::{Quantity, energy::KilowattHours, rate::KilowattHourRate};

pub type Cost = Quantity<0, 0, 1>;

impl Cost {
    pub const ONE_CENT: Self = Self(0.01);
}

impl Display for Cost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} €", self.0)
    }
}

impl Debug for Cost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}€", self.0)
    }
}

impl Div<KilowattHours> for Cost {
    type Output = KilowattHourRate;

    fn div(self, rhs: KilowattHours) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}
