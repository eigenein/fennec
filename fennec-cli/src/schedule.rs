use std::collections::VecDeque;

use chrono::{DateTime, Local, TimeDelta};
use derive_more::IntoIterator;
use itertools::Itertools;

use crate::{ops::interval::Interval, prelude::*};

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

    /// Get the schedule total duration. Returns zero for empty schedule.
    #[must_use]
    pub fn duration(&self) -> TimeDelta {
        self.start().zip(self.end()).map_or(TimeDelta::zero(), |(start, end)| end - start)
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

    /// Returns [`true`], if any slot differs from the corresponding slot in the other schedule.
    ///
    /// Note: non-matched time slots make no difference, the schedules cover for each other.
    ///
    /// TODO: generalise [`Interval`] and add tests.
    pub fn differs_from_by(&self, other: &Self, differs: &impl Fn(&V, &V) -> bool) -> bool {
        let mut this = self.iter().peekable();
        let mut other = other.iter().peekable();
        loop {
            let (Some(this_slot), Some(other_slot)) = (this.peek(), other.peek()) else {
                break false;
            };
            if this_slot.interval.is_earlier_than(other_slot.interval) {
                this.next();
                continue;
            }
            if other_slot.interval.is_earlier_than(this_slot.interval) {
                other.next();
                continue;
            }
            if !this_slot.interval.contains(other_slot.interval)
                && !other_slot.interval.contains(this_slot.interval)
            {
                break true;
            }
            if differs(this_slot.value, other_slot.value) {
                break true;
            }
        }
    }

    /// Construct new schedule by mapping the schedule values.
    pub fn map<T>(&self, mut mapper: impl FnMut(&V) -> T) -> Schedule<T> {
        Schedule(self.0.iter().map(|slot| slot.map(&mut mapper)).collect())
    }

    /// Construct a new schedule by mapping the values, stopping at the first error.
    pub fn try_map<T>(&self, mut mapper: impl FnMut(&V) -> Result<T>) -> Result<Schedule<T>> {
        self.0
            .iter()
            .map(|slot| Ok(Slot { interval: slot.interval, value: mapper(&slot.value)? }))
            .collect::<Result<_>>()
            .map(Schedule)
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

    pub fn extend(&mut self, other: Self) -> Result {
        ensure!(
            self.end().zip(other.start()).is_none_or(|(end, start)| end == start),
            "the other schedule must start at this schedule end",
        );
        self.0.extend(other.0);
        Ok(())
    }

    /// Extend the schedule from an iterator over slots.
    pub fn extend_from_iter(
        &mut self,
        other: impl IntoIterator<Item = (Interval<DateTime<Local>>, V)>,
    ) -> Result {
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

    /// Remove intervals that ended before `now` and clamp the first remaining interval's start to `now`.
    pub fn advance_to(&mut self, timestamp: DateTime<Local>) {
        self.pop_before(timestamp);
        if let Some(first) = self.0.front_mut() {
            first.interval = first.interval.clamp_start_to(timestamp);
        }
    }

    /// Pop schedule slots that ended before the given timestamp.
    fn pop_before(&mut self, timestamp: DateTime<Local>) {
        while self.0.pop_front_if(|slot| slot.interval.end() <= timestamp).is_some() {}
    }
}

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct Slot<V> {
    pub interval: Interval<DateTime<Local>>,

    /// Payload of this schedule slot.
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
        // TODO: after [`Interval`] generalisation, use simpler type for the index:
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
