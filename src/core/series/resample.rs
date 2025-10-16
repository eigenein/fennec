use std::{
    iter::from_fn,
    ops::{Add, Div, Mul, Sub},
};

use chrono::{DateTime, DurationRound, Local, TimeDelta};
use itertools::Itertools;

impl<T> Resample for T where T: ?Sized {}

pub trait Resample {
    #[must_use]
    fn resample_by_interval<V>(
        self,
        interval: TimeDelta,
    ) -> impl Iterator<Item = (DateTime<Local>, V)>
    where
        Self: Iterator<Item = (DateTime<Local>, V)> + Sized,
        V: Copy + Add<V, Output = V> + Sub<V, Output = V> + Div<TimeDelta>,
        <V as Div<TimeDelta>>::Output: Copy + Mul<TimeDelta, Output = V>,
    {
        self.tuple_windows().flat_map(
            move |((left_timestamp, left_value), (right_timestamp, right_value))| {
                let dvdt = (right_value - left_value) / (right_timestamp - left_timestamp);
                let mut timestamp = left_timestamp.duration_trunc(interval).unwrap();
                from_fn(move || {
                    timestamp += interval;
                    if timestamp <= right_timestamp {
                        let dt = timestamp - left_timestamp;
                        Some((timestamp, left_value + dvdt * dt))
                    } else {
                        None
                    }
                })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::quantity::energy::KilowattHours;

    #[test]
    fn test_resample_by_interval() {
        let date = NaiveDate::from_ymd_opt(2025, 10, 11).unwrap();

        let series = vec![
            (
                // This one technically gets ignored:
                date.and_hms_opt(11, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
                KilowattHours::from(100.0),
            ),
            (
                date.and_hms_opt(11, 30, 0).unwrap().and_local_timezone(Local).unwrap(),
                KilowattHours::from(3.0),
            ),
            (
                // Skip 2 hours, it should still yield a point at 12:00:
                date.and_hms_opt(13, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
                KilowattHours::from(6.0),
            ),
            (
                date.and_hms_opt(14, 30, 0).unwrap().and_local_timezone(Local).unwrap(),
                KilowattHours::from(10.5),
            ),
        ];

        let series = series.into_iter().resample_by_interval(TimeDelta::hours(1)).collect_vec();
        assert_eq!(
            series,
            vec![
                (
                    date.and_hms_opt(12, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
                    KilowattHours::from(4.0)
                ),
                (
                    date.and_hms_opt(13, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
                    KilowattHours::from(6.0)
                ),
                (
                    date.and_hms_opt(14, 0, 0).unwrap().and_local_timezone(Local).unwrap(),
                    KilowattHours::from(9.0)
                ),
            ]
        );
    }
}
