use std::{
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

use chrono::{DateTime, DurationRound, Local, TimeDelta, Timelike};
use itertools::Itertools;

use crate::core::{point::Point, series::Series};

impl<V> Series<V> {
    /// Interpolate the time series and iterate over hours,
    /// yielding the hour timestamp and interpolated value.
    #[allow(clippy::type_repetition_in_bounds)]
    pub fn resample_hourly(&self) -> impl Iterator<Item = (DateTime<Local>, V)>
    where
        V: Copy,
        V: Add<V, Output = V>,
        V: Sub<V, Output = V>,
        V: Div<TimeDelta>,
        <V as Div<TimeDelta>>::Output: Mul<TimeDelta, Output = V>,
    {
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);

        self.0.iter().tuple_windows().filter(|((from, _), (to, _))| from.hour() != to.hour()).map(
            |((left_index, left_value), (right_index, right_value))| {
                let from: Point<V> = Point::new(*left_index, *left_value);
                let to: Point<V> = Point::new(*right_index, *right_value);
                let at = right_index.duration_trunc(ONE_HOUR).unwrap();
                (at, from.interpolate(to, at))
            },
        )
    }

    /// Group the points by hour and average the values.
    #[allow(clippy::type_repetition_in_bounds)]
    pub fn average_hourly(&self) -> [Option<V>; 24]
    where
        V: Copy,
        V: Sum,
        V: Div<f64, Output = V>,
    {
        let mut averages = [None; 24];
        self.0
            .iter()
            .into_group_map_by(|(index, _)| index.hour())
            .into_iter()
            .map(|(hour, points)| {
                if points.is_empty() {
                    (hour, None)
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    let n = points.len() as f64;
                    (hour, Some(points.into_iter().map(|(_, value)| *value).sum::<V>() / n))
                }
            })
            .for_each(|(index, value)| averages[index as usize] = value);
        averages
    }

    #[allow(clippy::type_repetition_in_bounds)]
    pub fn differentiate(
        &self,
    ) -> impl Iterator<Item = (DateTime<Local>, <V as Div<TimeDelta>>::Output)>
    where
        V: Copy,
        V: Sub<V, Output = V>,
        V: Div<TimeDelta>,
    {
        self.0.iter().tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (*from_index, (*to_value - *from_value) / (*to_index - *from_index))
        })
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::TimeZone;

    use super::*;
    use crate::quantity::energy::KilowattHours;

    #[test]
    fn test_resample_hourly() {
        let series = Series::from_iter([
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), KilowattHours::from(100.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), KilowattHours::from(150.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), KilowattHours::from(300.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), KilowattHours::from(400.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), KilowattHours::from(700.0)),
        ]);
        let resampled: Vec<_> = series.resample_hourly().collect();

        assert_eq!(resampled.len(), 2);

        assert_eq!(resampled[0].0, Local.with_ymd_and_hms(2025, 9, 21, 22, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[0].1.0, 200.0);

        assert_eq!(resampled[1].0, Local.with_ymd_and_hms(2025, 9, 21, 23, 0, 0).unwrap());
        assert_abs_diff_eq!(resampled[1].1.0, 500.0);
    }

    #[test]
    fn test_average_hourly() {
        let series = Series::from_iter([
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), 100.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), 150.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), 300.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), 400.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), 700.0),
        ]);
        assert_eq!(
            series.average_hourly(),
            [
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(125.0),
                Some(350.0),
                Some(700.0),
            ]
        );
    }

    #[test]
    fn test_differentiate() {
        let series = Series::from_iter([
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), KilowattHours::from(100.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), KilowattHours::from(150.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), KilowattHours::from(300.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), KilowattHours::from(400.0)),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), KilowattHours::from(700.0)),
        ]);
        let diff = series.differentiate().collect::<Series<_>>().into_iter().collect::<Vec<_>>();

        assert_eq!(diff.len(), 4);

        assert_eq!(diff[0].0, Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap());
        assert_abs_diff_eq!(diff[0].1.0, 200.0);

        assert_abs_diff_eq!(diff[1].1.0, 200.0);

        assert_abs_diff_eq!(diff[2].1.0, 400.0);

        assert_eq!(diff[3].0, Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap());
        assert_abs_diff_eq!(diff[3].1.0, 400.0);
    }
}
