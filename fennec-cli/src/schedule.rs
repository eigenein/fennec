use std::collections::VecDeque;

use chrono::{DateTime, Local};
use derive_more::{Deref, IntoIterator};

use crate::{prelude::*, quantity::time::Hours};

/// Half-open time interval.
///
/// TODO: switch to [`std::range::Range`].
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

    /// Restrict the interval start to the specified timestamp.
    pub fn clamp_start(mut self, to: DateTime<Local>) -> Self {
        if to > self.end {
            self.start = self.end;
        } else if to > self.start {
            self.start = to;
        }
        self
    }

    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}

#[must_use]
#[derive(Deref, IntoIterator)]
pub struct Schedule<V>(VecDeque<(Interval, V)>);

impl<V> Schedule<V> {
    /// Create new empty schedule.
    #[expect(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Build schedule from an iterable of slots.
    pub fn try_from_iter(iterable: impl IntoIterator<Item = (Interval, V)>) -> Result<Self> {
        let mut slots: VecDeque<_> = iterable.into_iter().collect();
        for [(lhs, _), (rhs, _)] in slots.make_contiguous().array_windows() {
            ensure!(lhs.end() == rhs.start(), "the schedule is non-continuous");
        }
        Ok(Self(slots))
    }

    /// Retain the schedule slots since the given moment in time.
    pub fn retain(&mut self, since: DateTime<Local>) {
        let remove_count = self.0.partition_point(|(interval, _)| interval.end <= since);
        self.0.drain(..remove_count);
    }

    /// Extend the schedule with the other schedule.
    pub fn extend(&mut self, other: Self) -> Result {
        if let Some(((lhs, _), (rhs, _))) = self.0.back().zip(other.0.front()) {
            ensure!(lhs.end() == rhs.start(), "schedule gap: `{:?}..{:?}`", lhs.end(), rhs.start());
        }
        self.0.extend(other.0);
        Ok(())
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
        assert_eq!(interval.clamp_start(to).start, start);

        // Target within the interval clamps to the target:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 45, 0).unwrap();
        assert_eq!(interval.clamp_start(to).start, to);

        // Target after the interval clamps to the end:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 55, 0).unwrap();
        assert_eq!(interval.clamp_start(to).start, end);
    }

    #[test]
    fn schedule_retain() {
        let first = Interval::new(
            Local.with_ymd_and_hms(2026, 5, 15, 16, 10, 0).unwrap(),
            Local.with_ymd_and_hms(2026, 5, 15, 16, 20, 0).unwrap(),
        );
        let second =
            Interval::new(first.end(), Local.with_ymd_and_hms(2026, 5, 15, 16, 30, 0).unwrap());

        let mut schedule = Schedule::try_from_iter([(first, 1), (second, 2)]).unwrap();

        schedule.retain(second.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0], (second, 2));
    }
}
