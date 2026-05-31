use chrono::TimeDelta;

use crate::quantity::{Format, Quantity};

pub type Hours<V = f64> = Quantity<V, 0, 0, 1, 0>;

impl<V> Format for Hours<V> {
    const SUFFIX: &str = "h";
    const PRECISION: usize = 2;
}

impl From<TimeDelta> for Hours {
    fn from(time_delta: TimeDelta) -> Self {
        Self(time_delta.as_seconds_f64() / 3600.0)
    }
}

impl Hours {
    /// Convert the duration to days.
    pub const fn days(self) -> f64 {
        self.0 / 24.0
    }
}
