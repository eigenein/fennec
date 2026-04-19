// TODO: may need to become integer.
quantity!(Watts, via: f64, suffix: "W", precision: 0);

impl From<fennec_modbus::contrib::Watts<i32>> for Watts {
    fn from(watts: fennec_modbus::contrib::Watts<i32>) -> Self {
        Self(f64::from(watts.0))
    }
}
