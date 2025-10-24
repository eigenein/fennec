use std::{
    cmp::Ordering,
    ops::{Add, Div},
};

use chrono::Timelike;
use itertools::Itertools;

impl<T> AggregateHourly for T where T: ?Sized {}

pub trait AggregateHourly {
    #[must_use]
    fn median_hourly<K, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Timelike,
        V: Copy + PartialOrd + Add<Output = V> + Div<f64, Output = V>,
    {
        let mut medians = [None; 24];
        for (hour, values) in self.into_group_map_by(|(timestamp, _)| timestamp.hour()) {
            medians[hour as usize] = values.into_iter().median();
        }
        medians
    }

    #[must_use]
    fn median<K, V>(self) -> Option<V>
    where
        Self: Sized + Iterator<Item = (K, V)>,
        V: Copy + Add<Output = V> + Div<f64, Output = V> + PartialOrd,
    {
        let mut values = self.map(|(_, value)| value).collect_vec();
        if values.is_empty() {
            None
        } else {
            values.sort_unstable_by(compare);
            let index = values.len() / 2;
            let index_value = *values.select_nth_unstable_by(index, compare).1;
            if values.len() % 2 == 1 {
                Some(index_value)
            } else {
                let leading_value = *values.select_nth_unstable_by(index - 1, compare).1;
                Some((leading_value + index_value) / 2.0)
            }
        }
    }
}

fn compare<V: PartialOrd>(lhs: &V, rhs: &V) -> Ordering {
    lhs.partial_cmp(rhs).unwrap()
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn test_median_odd() {
        let median = vec![((), 1.0), ((), 0.0), ((), 2.0)].into_iter().median().unwrap();
        assert_eq!(median, 1.0);
    }

    #[test]
    fn test_median_even() {
        let median = vec![((), 1.0), ((), 0.0), ((), 2.0), ((), 3.0)].into_iter().median().unwrap();
        assert_abs_diff_eq!(median, 1.5);
    }
}
