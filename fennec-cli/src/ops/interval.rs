use chrono::{DateTime, Local};

use crate::ops::RangeExclusive;

pub type Interval<Tz = Local> = RangeExclusive<DateTime<Tz>>;
