use std::ops::{Add, Div, Mul, Sub};

use chrono::{DateTime, Local, TimeDelta};

#[derive(Copy, Clone, derive_more::Constructor)]
pub struct Point<V, I = DateTime<Local>> {
    pub index: I,
    pub value: V,
}

impl<V> Point<V>
where
    V: Copy,
    V: Add<V, Output = V>,
    V: Sub<V, Output = V>,
    V: Div<TimeDelta>,
    <V as Div<TimeDelta>>::Output: Mul<TimeDelta, Output = V>,
{
    /// Perform linear interpolation between two time series points.
    pub fn interpolate(self, to: Self, at: DateTime<Local>) -> V {
        let change_per_second = (to.value - self.value) / (to.index - self.index);
        self.value + change_per_second * (at - self.index)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::TimeZone;

    use super::*;
    use crate::quantity::energy::KilowattHours;

    #[test]
    fn test_interpolate() {
        let from = Point::new(
            Local.with_ymd_and_hms(2025, 9, 21, 20, 30, 0).unwrap(),
            KilowattHours::from(1.0),
        );
        let to = Point::new(
            Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(),
            KilowattHours::from(3.0),
        );
        let middle = from.interpolate(to, Local.with_ymd_and_hms(2025, 9, 21, 21, 0, 0).unwrap());
        assert_abs_diff_eq!(middle.0, 2.0);
    }
}
