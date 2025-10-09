use std::{iter::Sum, ops::Div};

use chrono::Timelike;
use itertools::Itertools;

use crate::core::series::SumValues;

impl<T> AverageHourly for T where T: ?Sized {}

pub trait AverageHourly {
    fn average_hourly<K, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Timelike,
        V: Copy + Sum + Div<f64, Output = V>,
    {
        let mut averages = [None; 24];
        self.into_group_map_by(|(index, _)| index.hour())
            .into_iter()
            .map(|(hour, points)| {
                if points.is_empty() {
                    (hour, None)
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    let n = points.len() as f64;
                    (hour, Some(points.into_iter().sum_values() / n))
                }
            })
            .for_each(|(index, value)| averages[index as usize] = value);
        averages
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn test_average_hourly() {
        let series = vec![
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 30, 0).unwrap(), 100.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 21, 45, 0).unwrap(), 150.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 30, 0).unwrap(), 300.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 22, 45, 0).unwrap(), 400.0),
            (Local.with_ymd_and_hms(2025, 9, 21, 23, 30, 0).unwrap(), 700.0),
        ];
        assert_eq!(
            series.into_iter().average_hourly(),
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
}
