use std::{
    fmt::{Debug, Formatter},
    time::Duration,
};

use chrono::{DateTime, Local, TimeDelta};

use crate::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq)]
#[must_use]
pub struct Interval {
    /// Inclusive.
    pub start: DateTime<Local>,

    /// Exclusive.
    pub end: DateTime<Local>,
}

impl Debug for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl Interval {
    pub const fn new(start: DateTime<Local>, end: DateTime<Local>) -> Self {
        Self { start, end }
    }

    pub fn since(duration: TimeDelta) -> Self {
        let end = Local::now();
        Self { start: end - duration, end }
    }

    pub fn try_since(duration: Duration) -> Result<Self> {
        Ok(Self::since(TimeDelta::from_std(duration)?))
    }

    pub const fn with_start(mut self, start: DateTime<Local>) -> Self {
        self.start = start;
        self
    }

    pub const fn with_end(mut self, end: DateTime<Local>) -> Self {
        self.end = end;
        self
    }

    #[must_use]
    pub fn duration(self) -> TimeDelta {
        self.end - self.start
    }

    #[must_use]
    pub fn contains(self, other: DateTime<Local>) -> bool {
        (self.start <= other) && (other < self.end)
    }
}
