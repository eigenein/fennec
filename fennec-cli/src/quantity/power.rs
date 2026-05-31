use crate::quantity::{Format, Quantity};

pub type Watts<V = f64> = Quantity<V, 0, 1, 0, 0>;

impl Format for Watts {
    const SUFFIX: &str = "W";
}

impl From<fennec_modbus::contrib::Watts<i32>> for Watts {
    fn from(watts: fennec_modbus::contrib::Watts<i32>) -> Self {
        Self(f64::from(watts.0))
    }
}
