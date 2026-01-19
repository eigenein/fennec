use std::ops::{Add, Div, Range};

use chrono::{DateTime, TimeZone, Timelike};
use itertools::Itertools;

impl<T> Aggregate for T where T: ?Sized {}

pub trait Aggregate {
    #[must_use]
    fn median_hourly<Tz, V>(self) -> [Option<V>; 24]
    where
        Self: Sized + IntoIterator<Item = (Range<DateTime<Tz>>, V)>,
        Tz: TimeZone,
        V: Copy + Ord + Add<Output = V> + Div<f64, Output = V>,
        DateTime<Tz>: Copy,
    {
        let mut medians = [None; 24];
        for (hour, values) in self
            .into_iter()
            .filter(|(interval, _)| {
                // Filter out cross-hour values:
                (interval.start.date_naive() == interval.end.date_naive())
                    && (interval.start.hour() == interval.end.hour())
            })
            .into_group_map_by(|(time_range, _)| time_range.start.hour())
        {
            medians[hour as usize] = values.into_iter().map(|(_, value)| value).median();
        }
        medians
    }

    #[must_use]
    fn median<V>(self) -> Option<V>
    where
        Self: Sized + IntoIterator<Item = V>,
        V: Copy + Add<Output = V> + Div<f64, Output = V> + Ord,
    {
        let mut values = self.into_iter().collect_vec();
        if values.is_empty() {
            None
        } else {
            values.sort_unstable();
            let index = values.len() / 2;
            let index_value = *values.select_nth_unstable(index).1;
            if values.len() % 2 == 1 {
                Some(index_value)
            } else {
                let leading_value = *values.select_nth_unstable(index - 1).1;
                Some((leading_value + index_value) / 2.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use ordered_float::OrderedFloat;

    use super::*;

    #[test]
    #[expect(clippy::float_cmp)]
    fn test_median_odd() {
        let median = vec![1.0, 0.0, 2.0].into_iter().map(OrderedFloat).median().unwrap();
        assert_eq!(median.0, 1.0);
    }

    #[test]
    fn test_median_even() {
        let median = vec![1.0, 0.0, 2.0, 3.0].into_iter().map(OrderedFloat).median().unwrap();
        assert_abs_diff_eq!(median.0, 1.5);
    }
}
