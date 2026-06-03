use std::collections::VecDeque;

use chrono::{DateTime, Local};
use derive_more::IntoIterator;

use crate::{ops::chrono::Interval, prelude::*};

#[must_use]
#[derive(IntoIterator)]
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

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Retrieve the interval and value at the given index.
    ///
    /// Panics outside the bounds.
    pub fn get(&self, index: usize) -> (Interval, &V) {
        let (interval, value) = &self.0[index];
        (*interval, value)
    }

    /// Retrieve the mutable reference at the given index.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> &mut V {
        &mut self.0[index].1
    }

    /// Construct new schedule by mapping the schedule values.
    pub fn map<T>(&self, mapper: impl Fn(&V) -> T) -> Schedule<T> {
        Schedule(self.0.iter().map(|(interval, value)| (*interval, mapper(value))).collect())
    }

    /// Pop schedule slots that ended before the given timestamp.
    pub fn pop_before(&mut self, timestamp: DateTime<Local>) {
        while self.0.pop_front_if(|(interval, _)| interval.end() <= timestamp).is_some() {}
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
    fn schedule_pop_before() {
        let first = Interval::new(
            Local.with_ymd_and_hms(2026, 5, 15, 16, 10, 0).unwrap(),
            Local.with_ymd_and_hms(2026, 5, 15, 16, 20, 0).unwrap(),
        );
        let second =
            Interval::new(first.end(), Local.with_ymd_and_hms(2026, 5, 15, 16, 30, 0).unwrap());

        let mut schedule = Schedule::try_from_iter([(first, 1), (second, 2)]).unwrap();

        schedule.pop_before(second.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.get(0), (second, &2));
    }
}
