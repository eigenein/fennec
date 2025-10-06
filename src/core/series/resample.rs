use std::{
    iter::from_fn,
    ops::{Add, Div, Mul, Sub},
};

use chrono::{DateTime, DurationRound, TimeDelta, TimeZone};
use itertools::Itertools;

impl<T> ResampleHourly for T where T: ?Sized {}

pub trait ResampleHourly {
    /// Interpolate the time series and iterate over hours,
    /// yielding the hour beginning timestamp and interpolated value.
    fn resample_hourly<Tz, V, Dv, DvDt>(self) -> impl Iterator<Item = (DateTime<Tz>, V)>
    where
        Self: Iterator<Item = (DateTime<Tz>, V)> + Sized,
        Tz: TimeZone,
        V: Clone + Add<Dv, Output = V> + Sub<V, Output = Dv>,
        Dv: Div<TimeDelta, Output = DvDt>,
        DvDt: Clone + Mul<TimeDelta, Output = Dv>,
    {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);

        self.tuple_windows().flat_map(|((left_key, left_value), (right_key, right_value))| {
            let dv = (right_value - left_value.clone()) / (right_key.clone() - left_key.clone());
            let mut at = left_key.clone().duration_trunc(ONE_HOUR).unwrap();
            from_fn(move || {
                at += ONE_HOUR;
                if at <= right_key {
                    Some((
                        at.clone(),
                        left_value.clone() + dv.clone() * (at.clone() - left_key.clone()),
                    ))
                } else {
                    None
                }
            })
        })
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

    /// Verify that many-hour intervals get upsampled.
    #[test]
    fn test_resample_hourly_from_longer_interval() {
        let series = vec![
            (Local.with_ymd_and_hms(2025, 10, 6, 10, 30, 0).unwrap(), KilowattHours::from(1.0)),
            (Local.with_ymd_and_hms(2025, 10, 6, 12, 30, 0).unwrap(), KilowattHours::from(5.0)),
        ];
        let resampled: Vec<_> = series.into_iter().resample_hourly().collect();

        assert_eq!(resampled.len(), 2);

        assert_eq!(resampled[0].0, Local.with_ymd_and_hms(2025, 10, 6, 11, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[0].1.0, 2.0);

        assert_eq!(resampled[1].0, Local.with_ymd_and_hms(2025, 10, 6, 12, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[1].1.0, 4.0);
    }

    /// Verify that the right boundary gets included, whilst the left boundary gets excluded.
    #[test]
    fn test_resample_hourly_boundaries() {
        let series = vec![
            (Local.with_ymd_and_hms(2025, 10, 6, 11, 0, 0).unwrap(), KilowattHours::from(1.0)),
            (Local.with_ymd_and_hms(2025, 10, 6, 12, 0, 0).unwrap(), KilowattHours::from(2.0)),
            (Local.with_ymd_and_hms(2025, 10, 6, 13, 0, 0).unwrap(), KilowattHours::from(3.0)),
        ];
        let resampled: Vec<_> = series.into_iter().resample_hourly().collect();

        assert_eq!(resampled.len(), 2);

        assert_eq!(resampled[0].0, Local.with_ymd_and_hms(2025, 10, 6, 12, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[0].1.0, 2.0);

        assert_eq!(resampled[1].0, Local.with_ymd_and_hms(2025, 10, 6, 13, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[1].1.0, 3.0);
    }
}
