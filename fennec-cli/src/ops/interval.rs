use chrono::{DateTime, Local};

use crate::ops::range;

pub type Interval<Tz = Local> = range::Exclusive<DateTime<Tz>>;
