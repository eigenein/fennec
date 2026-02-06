use std::{
    fmt::{Debug, Formatter},
    ops::Sub,
};

use chrono::{DateTime, Local};

pub type Interval<Tz = Local> = RangeExclusive<DateTime<Tz>>;

#[must_use]
#[derive(Copy, Clone)]
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
    pub const fn new(range: std::ops::Range<T>) -> Self {
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

impl<T: Copy + Sub> RangeExclusive<T> {
    #[must_use]
    pub fn len(self) -> <T as Sub>::Output {
        self.end - self.start
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
    pub start: T,
    pub end: T,
}

impl<T: Copy> RangeInclusive<T> {
    pub const fn new(range: std::ops::RangeInclusive<T>) -> Self {
        Self { start: *range.start(), end: *range.end() }
    }
}

impl<T: Copy + PartialOrd> RangeInclusive<T> {
    #[must_use]
    pub fn contains(self, other: T) -> bool {
        (self.start <= other) && (other <= self.end)
    }
}
