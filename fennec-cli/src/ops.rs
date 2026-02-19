mod interval;

use std::fmt::{Debug, Formatter};

use chrono::{DateTime, TimeZone};

pub use self::interval::Interval;
use crate::quantity::time::Hours;

#[must_use]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct RangeExclusive<T: Copy> {
    pub start: T,
    pub end: T,
}

impl<T: Copy + Debug> Debug for RangeExclusive<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl<T: Copy> RangeExclusive<T> {
    pub const fn from_std(range: std::ops::Range<T>) -> Self {
        Self { start: range.start, end: range.end }
    }

    pub const fn with_start(mut self, start: T) -> Self {
        self.start = start;
        self
    }

    pub const fn with_end(mut self, end: T) -> Self {
        self.end = end;
        self
    }
}

impl<Tz> RangeExclusive<DateTime<Tz>>
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: Copy,
{
    pub fn hours(self) -> Hours {
        (self.end - self.start).into()
    }
}

impl<T: Copy + PartialOrd> RangeExclusive<T> {
    #[must_use]
    pub fn contains(self, other: T) -> bool {
        (self.start <= other) && (other < self.end)
    }
}

#[must_use]
#[derive(Copy, Clone)]
pub struct RangeInclusive<T: Copy> {
    pub min: T,
    pub max: T,
}

impl<T: Copy> From<std::ops::RangeInclusive<T>> for RangeInclusive<T> {
    fn from(range: std::ops::RangeInclusive<T>) -> Self {
        Self::from_std(range)
    }
}

impl<T: Copy> RangeInclusive<T> {
    pub const fn from_std(range: std::ops::RangeInclusive<T>) -> Self {
        Self { min: *range.start(), max: *range.end() }
    }
}

impl<T: Copy + PartialOrd> RangeInclusive<T> {
    #[must_use]
    pub fn contains(self, other: T) -> bool {
        (self.min <= other) && (other <= self.max)
    }
}
