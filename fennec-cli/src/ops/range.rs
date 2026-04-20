use std::fmt::{Debug, Formatter};

use chrono::{DateTime, TimeZone};

use crate::quantity::time::Hours;

#[must_use]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Exclusive<T> {
    pub start: T,
    pub end: T,
}

impl<T: Debug> Debug for Exclusive<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl<T> Exclusive<T> {
    /// TODO: convert to builder.
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

    #[must_use]
    pub fn contains(self, other: T) -> bool
    where
        T: Copy + PartialOrd,
    {
        (self.start <= other) && (other < self.end)
    }
}

impl<Tz> Exclusive<DateTime<Tz>>
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: Copy,
{
    pub fn hours(self) -> Hours {
        (self.end - self.start).into()
    }
}

#[must_use]
#[derive(Copy, Clone, derive_more::Debug)]
#[debug("{min:?}..={max:?}")]
pub struct Inclusive<T> {
    pub min: T,
    pub max: T,
}

impl<T: Copy> From<std::ops::RangeInclusive<T>> for Inclusive<T> {
    fn from(range: std::ops::RangeInclusive<T>) -> Self {
        Self::from_std(range)
    }
}

impl<T> Inclusive<T> {
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
