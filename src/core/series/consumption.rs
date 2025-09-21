use std::ops::{Add, Div, Mul, Sub};

use chrono::{DateTime, DurationRound, Local, TimeDelta, Timelike};
use itertools::Itertools;

use crate::{
    core::{point::Point, series::Series},
    prelude::*,
};

impl<V> Series<V> {
    pub fn resample_hourly(&self) -> impl Iterator<Item = Result<(DateTime<Local>, V)>>
    where
        V: Copy,
        V: Add<V, Output = V>,
        V: Sub<V, Output = V>,
        V: Mul<f64, Output = V>,
        V: Div<f64, Output = V>,
    {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);

        self.0.iter().tuple_windows().filter(|((from, _), (to, _))| from.hour() != to.hour()).map(
            |((left_index, left_value), (right_index, right_value))| {
                let from: Point<V> = Point::new(*left_index, *left_value);
                let to: Point<V> = Point::new(*right_index, *right_value);
                let at = right_index.duration_trunc(ONE_HOUR)?;
                Ok((at, from.interpolate(to, at)))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_resample_hourly() -> Result {
        let series = Series::from_iter([
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), 100.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), 150.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), 300.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), 400.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), 700.0),
        ]);
        let resampled: Vec<_> = series.resample_hourly().collect::<Result<_>>()?;

        assert_eq!(resampled.len(), 2);

        assert_eq!(resampled[0].0, Local.with_ymd_and_hms(2025, 9, 21, 22, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[0].1, 200.0);

        assert_eq!(resampled[1].0, Local.with_ymd_and_hms(2025, 9, 21, 23, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[1].1, 500.0);

        Ok(())
    }
}
