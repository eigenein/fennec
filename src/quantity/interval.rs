use std::fmt::{Debug, Formatter};

use chrono::{DateTime, Local};

#[derive(Copy, Clone, Eq, PartialEq)]
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

    pub fn contains(self, other: DateTime<Local>) -> bool {
        (self.start <= other) && (other < self.end)
    }
}
