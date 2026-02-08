use std::iter::once;

use chrono::{DateTime, Local, TimeDelta, TimeZone};

use crate::ops::RangeExclusive;

pub type Interval<Tz = Local> = RangeExclusive<DateTime<Tz>>;

impl<Tz> Interval<Tz>
where
    Tz: TimeZone + 'static,
    <Tz as TimeZone>::Offset: Copy,
{
    pub fn split(self, n: u16) -> impl Iterator<Item = Self> {
        let inner_delta = TimeDelta::minutes(self.len().num_minutes() / (i64::from(n) + 1));
        let last_interval = self.with_start(self.start + inner_delta * i32::from(n));
        (0..n)
            .map(move |i| {
                let start = self.start + inner_delta * i32::from(i);
                Self { start, end: start + inner_delta }
            })
            .chain(once(last_interval))
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_zero_splits() {
        let interval = Interval {
            start: Local.with_ymd_and_hms(2025, 2, 8, 15, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(2025, 2, 8, 16, 0, 0).unwrap(),
        };
        assert_eq!(interval.split(0).collect_vec(), vec![interval]);
    }

    #[test]
    fn test_multiple_splits() {
        let start = Local.with_ymd_and_hms(2025, 2, 8, 15, 0, 0).unwrap();
        let middle = Local.with_ymd_and_hms(2025, 2, 8, 15, 30, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 2, 8, 16, 0, 0).unwrap();
        let interval = Interval { start, end };
        assert_eq!(
            interval.split(1).collect_vec(),
            vec![Interval { start, end: middle }, Interval { start: middle, end }]
        );
    }
}
