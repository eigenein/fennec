use chrono::{DateTime, Local, TimeZone};

use crate::quantity::time::Hours;

/// Half-open time interval.
#[derive(Copy, Clone)]
pub struct Interval<Tz: TimeZone = Local>
where
    DateTime<Tz>: Copy,
{
    pub start: DateTime<Tz>,
    pub end: DateTime<Tz>,
}

impl<Tz> Interval<Tz>
where
    DateTime<Tz>: Copy,
    Tz: TimeZone,
{
    pub fn with_start(self, start: DateTime<Tz>) -> Self {
        Self { start, end: self.end }
    }

    pub fn contains(self, other: DateTime<Tz>) -> bool {
        (self.start <= other) && (other < self.end)
    }

    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}
