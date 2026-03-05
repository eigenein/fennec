mod interval;

use std::fmt::{Debug, Formatter};

use chrono::{DateTime, TimeZone};

pub use self::interval::Interval;
use crate::quantity::time::Hours;

#[must_use]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct RangeExclusive<T> {
    pub start: T,
    pub end: T,
}

impl<T: Debug> Debug for RangeExclusive<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl<T> RangeExclusive<T> {
    pub const fn from_std(range: std::ops::Range<T>) -> Self
    where
        T: Copy,
    {
        Self { start: range.start, end: range.end }
    }

    pub const fn with_start(mut self, start: T) -> Self
    where
        T: Copy,
    {
        self.start = start;
        self
    }

    pub const fn with_end(mut self, end: T) -> Self
    where
        T: Copy,
    {
        self.end = end;
        self
    }

    #[must_use]
    pub fn contains(self, other: T) -> bool
    where
        T: Copy + PartialOrd,
    {
        (self.start <= other) && (other < self.end)
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

#[must_use]
#[derive(Copy, Clone)]
pub struct RangeInclusive<T> {
    pub min: T,
    pub max: T,
}

impl<T: Copy> From<std::ops::RangeInclusive<T>> for RangeInclusive<T> {
    fn from(range: std::ops::RangeInclusive<T>) -> Self {
        Self::from_std(range)
    }
}

impl<T> RangeInclusive<T> {
    pub const fn from_std(range: std::ops::RangeInclusive<T>) -> Self
    where
        T: Copy,
    {
        Self { min: *range.start(), max: *range.end() }
    }

    #[must_use]
    pub fn contains(self, other: T) -> bool
    where
        T: Copy + PartialOrd,
    {
        (self.min <= other) && (other <= self.max)
    }
}
