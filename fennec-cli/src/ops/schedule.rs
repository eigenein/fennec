use chrono::{DateTime, Local, TimeZone};

use crate::{ops::range, quantity::time::Hours};

/// Half-open time interval.
pub type Interval<Tz = Local> = range::Exclusive<DateTime<Tz>>;

impl<Tz: TimeZone> Interval<Tz> {
    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}
