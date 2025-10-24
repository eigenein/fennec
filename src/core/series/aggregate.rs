use std::ops::{Add, Div};

use chrono::Timelike;
use itertools::Itertools;

impl<T> AggregateHourly for T where T: ?Sized {}

pub trait AggregateHourly {
    fn average_hourly<K, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Timelike,
        V: Copy + Add<Output = V> + Div<f64, Output = V>,
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
        let time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let averages = vec![(time, 10.0), (time, 2.0)].into_iter().average_hourly();
        assert_eq!(&averages[0..23], [None; 23]);
        assert_eq!(averages[23], Some(6.0));
    }
}
