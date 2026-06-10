use std::collections::VecDeque;

use chrono::{DateTime, Local};
use derive_more::IntoIterator;
use itertools::Itertools;

use crate::{ops::chrono::Interval, prelude::*};

#[must_use]
#[derive(IntoIterator)]
pub struct Schedule<V>(VecDeque<Slot<V>>);

impl<V> Schedule<V> {
    /// Create new empty schedule.
    #[expect(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(VecDeque::new())
    }

    #[must_use]
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get the first slot starting timestamp.
    #[must_use]
    pub fn start(&self) -> Option<DateTime<Local>> {
        self.0.front().map(|slot| slot.interval.start())
    }

    /// Get the last slot end timestamp, exclusive.
    #[must_use]
    pub fn end(&self) -> Option<DateTime<Local>> {
        self.0.back().map(|slot| slot.interval.end())
    }

    /// Retrieve the interval and value at the given index.
    ///
    /// Panics outside the bounds.
    pub fn get(&self, index: usize) -> Slot<&V> {
        self.0[index].as_ref()
    }

    /// Retrieve the mutable reference at the given index.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> &mut V {
        &mut self.0[index].value
    }

    /// Construct new schedule by mapping the schedule values.
    pub fn map<T>(&self, mapper: impl Fn(&V) -> T) -> Schedule<T> {
        Schedule(self.0.iter().map(|slot| slot.map(&mapper)).collect())
    }

    pub fn zip_eq<T>(self, iterable: impl IntoIterator<Item = T>) -> Schedule<(V, T)> {
        Schedule(
            self.0
                .into_iter()
                .zip_eq(iterable)
                .map(|(lhs, rhs)| Slot { interval: lhs.interval, value: (lhs.value, rhs) })
                .collect(),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = Slot<&V>> {
        self.0.iter().map(Slot::as_ref)
    }

    /// Pop schedule slots that ended before the given timestamp.
    pub fn pop_before(&mut self, timestamp: DateTime<Local>) {
        while self.0.pop_front_if(|slot| slot.interval.end() <= timestamp).is_some() {}
    }

    pub fn extend(&mut self, other: Self) -> Result {
        ensure!(
            self.end().zip(other.start()).is_none_or(|(end, start)| end == start),
            "the other schedule must start at this schedule end",
        );
        self.0.extend(other.0);
        Ok(())
    }

    /// Extend the schedule from an iterator over slots.
    pub fn extend_from_iter(&mut self, other: impl IntoIterator<Item = (Interval, V)>) -> Result {
        for (interval, value) in other {
            let current_end = self.end();
            ensure!(
                current_end.is_none_or(|current_end| current_end == interval.start()),
                "trying to push `{interval:?}` on top of `{current_end:?}`",
            );
            self.0.push_back(Slot { interval, value });
        }
        Ok(())
    }
}

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct Slot<V> {
    pub interval: Interval,
    pub value: V,
}

impl<V> Slot<V> {
    pub fn map<T>(&self, mapper: impl FnOnce(&V) -> T) -> Slot<T> {
        Slot { interval: self.interval, value: mapper(&self.value) }
    }

    pub const fn as_ref(&self) -> Slot<&V> {
        Slot { interval: self.interval, value: &self.value }
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

        let mut schedule = Schedule::new();
        schedule.extend_from_iter([(first, 1), (second, 2)]).unwrap();

        schedule.pop_before(second.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.get(0), Slot { interval: second, value: &2 });
    }
}
