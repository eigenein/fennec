use std::ops::{Add, Div, Mul, Sub};

use chrono::{DateTime, DurationRound, TimeDelta, TimeZone, Timelike};
use itertools::Itertools;

impl<T> Resample for T where T: ?Sized {}

pub trait Resample {
    #[must_use]
    fn resample<K, V>(self, sample: fn(&K, &K) -> Option<K>) -> impl Iterator<Item = (K, V)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Clone + Sub<K>,
        V: Clone + Add<V, Output = V> + Sub<V>,
        <V as Sub<V>>::Output: Div<<K as Sub<K>>::Output>,
        <<V as Sub<V>>::Output as Div<<K as Sub<K>>::Output>>::Output:
            Mul<<K as Sub<K>>::Output, Output = V>,
    {
        self.tuple_windows().filter_map(
            move |((left_key, left_value), (right_key, right_value))| {
                sample(&left_key, &right_key).map(|key| {
                    let dvdk = (right_value - left_value.clone()) / (right_key - left_key.clone());
                    let dk = key.clone() - left_key;
                    (key, left_value + dvdk * dk)
                })
            },
        )
    }
}

pub fn resample_hourly<Tz: TimeZone>(lhs: &DateTime<Tz>, rhs: &DateTime<Tz>) -> Option<DateTime<Tz>>
where
    DateTime<Tz>: Copy,
{
    ((lhs.date_naive() != rhs.date_naive()) || (lhs.hour() != rhs.hour()))
        .then(|| rhs.duration_trunc(TimeDelta::hours(1)).unwrap())
}

pub fn resample_daily<Tz: TimeZone>(lhs: &DateTime<Tz>, rhs: &DateTime<Tz>) -> Option<DateTime<Tz>>
where
    DateTime<Tz>: Copy,
{
    (lhs.date_naive() != rhs.date_naive()).then(|| rhs.duration_trunc(TimeDelta::days(1)).unwrap())
}

#[cfg(test)]
mod tests {
    use chrono::{Local, NaiveDate};

    use super::*;

    #[test]
    fn test_resample() {
        let series = [(1.0, 0.0), (1.5, 100.0), (2.5, 200.0), (2.9, 300.0)];
        let series = series
            .into_iter()
            .resample(|lhs: &f64, rhs: &f64| (lhs.trunc() != rhs.trunc()).then(|| rhs.trunc()))
            .collect_vec();
        assert_eq!(series, [(2.0, 150.0)]);
    }

    #[test]
    fn test_resample_hourly() {
        let date = NaiveDate::from_ymd_opt(2025, 10, 11).unwrap();

        let lhs = date.and_hms_opt(19, 55, 0).unwrap().and_local_timezone(Local).unwrap();
        assert!(resample_hourly(&lhs, &lhs).is_none());

        let rhs = date.and_hms_opt(20, 5, 0).unwrap().and_local_timezone(Local).unwrap();
        let expected = date.and_hms_opt(20, 0, 0).unwrap().and_local_timezone(Local).unwrap();
        assert_eq!(resample_hourly(&lhs, &rhs), Some(expected));
    }

    #[test]
    fn test_resample_daily() {
        let lhs = NaiveDate::from_ymd_opt(2025, 10, 11)
            .unwrap()
            .and_hms_opt(19, 55, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        assert!(resample_daily(&lhs, &lhs).is_none());

        let rhs = NaiveDate::from_ymd_opt(2025, 10, 12)
            .unwrap()
            .and_hms_opt(2, 15, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 10, 12)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        assert_eq!(resample_daily(&lhs, &rhs), Some(expected));
    }
}
