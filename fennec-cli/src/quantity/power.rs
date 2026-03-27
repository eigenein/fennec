quantity!(Watts, via: f64, suffix: "W", precision: 0);

impl From<i32> for Watts {
    fn from(watts: i32) -> Self {
        Self(f64::from(watts))
    }
}
