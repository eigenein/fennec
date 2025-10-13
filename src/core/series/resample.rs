use std::ops::{Add, Div, Mul, Sub};

use chrono::{DateTime, DurationRound, TimeDelta, TimeZone};
use itertools::Itertools;

impl<T> Resample for T where T: ?Sized {}

pub trait Resample {
    #[must_use]
    fn resample<K, V>(self, sample: impl Fn(&K, &K) -> Option<K>) -> impl Iterator<Item = (K, V)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub<K>,
        V: Copy + Add<V, Output = V> + Sub<V, Output = V> + Div<<K as Sub<K>>::Output>,
        <V as Div<<K as Sub<K>>::Output>>::Output: Mul<<K as Sub<K>>::Output, Output = V>,
    {
        self.tuple_windows().filter_map(
            move |((left_key, left_value), (right_key, right_value))| {
                sample(&left_key, &right_key).map(|key| {
                    let dvdk = (right_value - left_value) / (right_key - left_key);
                    let dk = key - left_key;
                    (key, left_value + dvdk * dk)
                })
            },
        )
    }
}

pub fn resample_on_time_delta<Tz>(
    time_delta: TimeDelta,
) -> impl Fn(&DateTime<Tz>, &DateTime<Tz>) -> Option<DateTime<Tz>>
where
    DateTime<Tz>: Copy,
    Tz: TimeZone,
{
    move |lhs, rhs| {
        let lhs = lhs.duration_trunc(time_delta).unwrap();
        let rhs = rhs.duration_trunc(time_delta).unwrap();
        (lhs != rhs).then_some(rhs)
    }
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
        let resample = resample_on_time_delta(TimeDelta::hours(1));

        let lhs = date.and_hms_opt(19, 55, 0).unwrap().and_local_timezone(Local).unwrap();
        assert!(resample(&lhs, &lhs).is_none());

        let rhs = date.and_hms_opt(20, 5, 0).unwrap().and_local_timezone(Local).unwrap();
        let expected = date.and_hms_opt(20, 0, 0).unwrap().and_local_timezone(Local).unwrap();
        assert_eq!(resample(&lhs, &rhs), Some(expected));
    }

    #[test]
    fn test_resample_daily() {
        let resample = resample_on_time_delta(TimeDelta::days(1));
        let lhs = NaiveDate::from_ymd_opt(2025, 10, 11)
            .unwrap()
            .and_hms_opt(19, 55, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        assert!(resample(&lhs, &lhs).is_none());

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
        assert_eq!(resample(&lhs, &rhs), Some(expected));
    }
}
