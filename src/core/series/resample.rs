use std::ops::{Add, Div, Mul, Sub};

use chrono::{DurationRound, TimeDelta, Timelike};
use itertools::Itertools;

impl<K, V, T> ResampleHourly<K, V> for T where T: ?Sized {}

pub trait ResampleHourly<K, V> {
    /// Interpolate the time series and iterate over hours,
    /// yielding the hour beginning timestamp and interpolated value.
    fn resample_hourly<Dk, DvDk>(self) -> impl Iterator<Item = (K, V)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Clone + DurationRound + Timelike + Sub<K, Output = Dk>,
        V: Clone + Add<V, Output = V> + Sub<V, Output = V> + Div<Dk, Output = DvDk>,
        DvDk: Mul<Dk, Output = V>,
    {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);

        // FIXME: what if there is an interval longer that one hour?
        self.tuple_windows().filter(|((from, _), (to, _))| from.hour() != to.hour()).map(
            |((left_key, left_value), (right_key, right_value))| {
                let at = right_key.clone().duration_trunc(ONE_HOUR).unwrap();
                let dv = (right_value - left_value.clone()) / (right_key - left_key.clone());
                (at.clone(), left_value + dv * (at - left_key))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::{Local, TimeZone};

    use super::*;
    use crate::quantity::energy::KilowattHours;

    #[test]
    fn test_resample_hourly() {
        let series = vec![
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), KilowattHours::from(100.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), KilowattHours::from(150.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), KilowattHours::from(300.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), KilowattHours::from(400.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), KilowattHours::from(700.0)),
        ];
        let resampled: Vec<_> = series.into_iter().resample_hourly().collect();

        assert_eq!(resampled.len(), 2);

        assert_eq!(resampled[0].0, Local.with_ymd_and_hms(2025, 9, 21, 22, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[0].1.0, 200.0);

        assert_eq!(resampled[1].0, Local.with_ymd_and_hms(2025, 9, 21, 23, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[1].1.0, 500.0);
    }
}
