use std::ops::Add;

use chrono::{DateTime, Days, Local, TimeZone};

use crate::ops::RangeExclusive;

pub type Interval<Tz = Local> = RangeExclusive<DateTime<Tz>>;

impl<Tz> Add<Days> for Interval<Tz>
where
    Tz: TimeZone,
    DateTime<Tz>: Copy,
{
    type Output = Self;

    /// Offset the entire interval with the [`Days`].
    fn add(self, days: Days) -> Self::Output {
        Self {
            start: self.start.checked_add_days(days).unwrap(),
            end: self.end.checked_add_days(days).unwrap(),
        }
    }
}
