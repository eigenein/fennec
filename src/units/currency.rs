use ordered_float::OrderedFloat;

use crate::units::Quantity;

pub type Cost = Quantity<OrderedFloat<f64>, 0, 0, 0, 1>;

impl Cost {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
}
