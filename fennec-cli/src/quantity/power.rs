quantity!(Watts, via: f64, suffix: "W", precision: 0);

impl From<fennec_modbus::contrib::Watts> for Watts {
    fn from(watts: fennec_modbus::contrib::Watts) -> Self {
        Self(f64::from(watts.0))
    }
}
