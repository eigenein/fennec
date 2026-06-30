use chrono::{DateTime, Local};

use crate::quantity::time::Hours;

/// Half-open time interval.
///
/// TODO: could become a wrapper around [`std::range::Range`].
/// TODO: I could generalise index for simpler testing.
#[must_use]
#[derive(Copy, Clone, PartialEq, Eq, derive_more::Debug)]
#[debug("{start:?}..{end:?}")]
pub struct Interval {
    start: DateTime<Local>,
    end: DateTime<Local>,
}

impl Interval {
    pub fn new(start: DateTime<Local>, end: DateTime<Local>) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    #[must_use]
    pub const fn start(self) -> DateTime<Local>
    where
        Self: Copy,
    {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> DateTime<Local>
    where
        Self: Copy,
    {
        self.end
    }

    /// Returns [`true`] if the interval ends earlier than the other interval starts.
    #[must_use]
    pub fn is_earlier_than(self, other: Self) -> bool
    where
        Self: Copy,
    {
        self.end <= other.start
    }

    /// Returns [`true`] if the interval fully contains the other interval.
    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        (self.start <= other.start) && (other.end <= self.end)
    }

    /// Restrict the interval start to the specified timestamp.
    pub fn clamp_start_to(mut self, timestamp: DateTime<Local>) -> Self {
        if timestamp > self.end {
            self.start = self.end;
        } else if timestamp > self.start {
            self.start = timestamp;
        }
        self
    }

    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn interval_clamp_start() {
        let start = Local.with_ymd_and_hms(2026, 5, 15, 14, 40, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 5, 15, 14, 50, 0).unwrap();
        let interval = Interval { start, end };

        // Target before the interval does not change the interval:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 30, 0).unwrap();
        assert_eq!(interval.clamp_start_to(to).start, start);

        // Target within the interval clamps to the target:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 45, 0).unwrap();
        assert_eq!(interval.clamp_start_to(to).start, to);

        // Target after the interval clamps to the end:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 55, 0).unwrap();
        assert_eq!(interval.clamp_start_to(to).start, end);
    }
}
