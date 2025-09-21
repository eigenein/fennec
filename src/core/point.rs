use chrono::{DateTime, Local};

#[derive(Copy, Clone, derive_more::Constructor)]
pub struct Point<V, I = DateTime<Local>> {
    pub index: I,
    pub value: V,
}

impl<V: From<f64> + Into<f64>> Point<V> {
    /// Perform linear interpolation between two time series points.
    pub fn interpolate(self, to: Self, at: DateTime<Local>) -> V {
        let from_value = self.value.into();
        let change_per_second =
            (to.value.into() - from_value) / (to.index - self.index).as_seconds_f64();
        change_per_second.mul_add((at - self.index).as_seconds_f64(), from_value).into()
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_interpolate() {
        let from = Point::new(Local.with_ymd_and_hms(2025, 9, 21, 20, 30, 0).unwrap(), 1.0);
        let to = Point::new(Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), 3.0);
        let middle = from.interpolate(to, Local.with_ymd_and_hms(2025, 9, 21, 21, 0, 0).unwrap());
        assert_abs_diff_eq!(middle, 2.0);
    }
}
