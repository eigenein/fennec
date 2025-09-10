use ordered_float::OrderedFloat;

use crate::units::Quantity;

pub type Cost = Quantity<OrderedFloat<f64>, 0, 0, 0, 1>;

impl From<f64> for Cost {
    fn from(value: f64) -> Self {
        Self::from(OrderedFloat(value))
    }
}
