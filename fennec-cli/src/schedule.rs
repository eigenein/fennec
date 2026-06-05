use std::collections::VecDeque;

use chrono::{DateTime, Local};
use derive_more::IntoIterator;

use crate::{ops::chrono::Interval, prelude::*};

#[must_use]
#[derive(IntoIterator)]
pub struct Schedule<V> {
    start: DateTime<Local>,

    /// TODO: only store the interval end points.
    #[into_iterator]
    slots: VecDeque<(Interval, V)>,
}

impl<V> Schedule<V> {
    /// Create new empty schedule.
    pub const fn new(start: DateTime<Local>) -> Self {
        Self { start, slots: VecDeque::new() }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The schedule ends when the last interval ends, or at the start if empty.
    #[must_use]
    pub fn end(&self) -> DateTime<Local> {
        self.slots.back().map_or(self.start, |(interval, _)| interval.end())
    }

    /// Retrieve the interval and value at the given index.
    ///
    /// Panics outside the bounds.
    pub fn get(&self, index: usize) -> (Interval, &V) {
        let (interval, value) = &self.slots[index];
        (*interval, value)
    }

    /// Retrieve the mutable reference at the given index.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> &mut V {
        &mut self.slots[index].1
    }

    /// Construct new schedule by mapping the schedule values.
    pub fn map<T>(&self, mapper: impl Fn(&V) -> T) -> Schedule<T> {
        Schedule {
            start: self.start,
            slots: self.slots.iter().map(|(interval, value)| (*interval, mapper(value))).collect(),
        }
    }

    /// Pop schedule slots that ended before the given timestamp.
    pub fn pop_before(&mut self, timestamp: DateTime<Local>) {
        while self.slots.pop_front_if(|(interval, _)| interval.end() <= timestamp).is_some() {}
    }

    /// Extend the schedule with the other schedule.
    pub fn extend(&mut self, other: impl IntoIterator<Item = (Interval, V)>) -> Result {
        for (interval, value) in other {
            let current_end = self.end();
            ensure!(
                interval.start() == current_end,
                "trying to push `{interval:?}` on top of `{current_end:?}`",
            );
            self.slots.push_back((interval, value));
        }
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

        let mut schedule = Schedule::new(first.start());
        schedule.extend([(first, 1), (second, 2)]).unwrap();

        schedule.pop_before(second.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.get(0), (second, &2));
    }
}
