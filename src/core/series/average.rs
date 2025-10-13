use std::{hash::Hash, iter::Sum, ops::Div};

use chrono::Timelike;
use itertools::Itertools;

use crate::core::series::SumValues;

impl<T> AverageHourly for T where T: ?Sized {}

pub trait AverageHourly {
    fn average_hourly<K, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Hash + Eq + Timelike,
        V: Copy + Sum + Div<f64, Output = V>,
    {
        let mut averages = [None; 24];
        self.into_group_map_by(|(key, _)| key.hour())
            .into_iter()
            .map(|(hour, points)| {
                assert!(!points.is_empty());

                #[allow(clippy::cast_precision_loss)]
                let len = points.len() as f64;

                // Average the points within similar hours:
                (hour, points.into_iter().sum_values() / len)
            })
            .for_each(|(hour, value)| averages[hour as usize] = Some(value));
        averages
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;

    #[test]
    fn test_average_hourly() {
        let series = vec![
            (NaiveTime::from_hms_opt(23, 0, 0).unwrap(), 10.0),
            (NaiveTime::from_hms_opt(23, 0, 0).unwrap(), 2.0),
        ];
        let averages = series.into_iter().average_hourly();
        assert_eq!(&averages[0..23], [None; 23]);
        assert_eq!(averages[23], Some(6.0));
    }
}
