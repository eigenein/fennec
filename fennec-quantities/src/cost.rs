use std::fmt::{Debug, Display, Formatter};

use ordered_float::OrderedFloat;

use crate::Quantity;

pub type Cost = Quantity<0, 0, 1>;

impl Cost {
    pub const ONE_CENT: Self = Self(OrderedFloat(0.01));
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
