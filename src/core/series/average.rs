use std::{
    hash::Hash,
    iter::Sum,
    ops::{Add, Div},
};

use chrono::Timelike;
use itertools::Itertools;

impl<T> AverageHourly for T where T: ?Sized {}

pub trait AverageHourly {
    fn average_hourly<K, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Hash + Eq + Timelike,
        V: Copy + Sum + Add<V, Output = V> + Div<f64, Output = V>,
    {
        let mut sums = [None; 24];
        let mut weights = [0_u32; 24];
        for (timestamp, value) in self {
            let hour = timestamp.hour() as usize;
            weights[hour] += 1;
            sums[hour] = Some(sums[hour].map_or(value, |sum| sum + value));
        }
        sums.into_iter()
            .zip(weights)
            .map(|(sum, weight)| sum.map(|sum| sum / f64::from(weight)))
            .collect_array()
            .unwrap()
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
