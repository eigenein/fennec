use std::ops::{Add, Div, Mul};

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

    fn hourly_quantiles<K, V>(self, p: f64) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Timelike,
        V: Copy + PartialOrd + Add<Output = V> + Mul<f64, Output = V>,
    {
        let mut hourly_quantiles = [None; 24];
        for (hour, values) in self.into_group_map_by(|(timestamp, _)| timestamp.hour()) {
            hourly_quantiles[hour as usize] = Some(values.into_iter().quantile(p));
        }
        hourly_quantiles
    }

    fn quantile<K, V>(self, p: f64) -> V
    where
        Self: Sized + Iterator<Item = (K, V)>,
        V: Copy + PartialOrd + Add<Output = V> + Mul<f64, Output = V>,
    {
        assert!((0.0..=1.0).contains(&p));

        let values = self
            .map(|(_, value)| value)
            .sorted_unstable_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
            .collect_vec();

        #[allow(clippy::cast_precision_loss)]
        let index = (values.len() - 1) as f64 * p;

        let lower_index = index.floor();
        let upper_index = index.ceil();

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let lower_value = values[lower_index as usize];

        #[allow(clippy::float_cmp)]
        if lower_index == upper_index {
            lower_value
        } else {
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            let upper_value = values[upper_index as usize];

            let weight = index - lower_index;
            lower_value * (1.0 - weight) + upper_value * weight
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use chrono::NaiveTime;

    use super::*;

    #[test]
    fn test_average_hourly() {
        let time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let averages = vec![(time, 10.0), (time, 2.0)].into_iter().average_hourly();
        assert_eq!(&averages[0..23], [None; 23]);
        assert_eq!(averages[23], Some(6.0));
    }

    #[test]
    fn test_exact_quantile() {
        let quantile = vec![((), 2.0), ((), 3.0), ((), 1.0)].into_iter().quantile(0.5);
        assert_eq!(quantile, 2.0);
    }

    #[test]
    fn test_interpolated_quantile() {
        let quantile = vec![((), 2.0), ((), 3.0), ((), 1.0), ((), 4.0)].into_iter().quantile(0.5);
        assert_abs_diff_eq!(quantile, 2.5);
    }

    #[test]
    fn test_hourly_quantiles() {
        let time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let averages =
            vec![(time, 2.0), (time, 3.0), (time, 1.0)].into_iter().hourly_quantiles(0.5);
        assert_eq!(&averages[0..23], [None; 23]);
        assert_eq!(averages[23], Some(2.0));
    }
}
