use std::{collections::VecDeque, fmt::Debug, ops::Sub};

use chrono::{DateTime, Local};
use derive_more::IntoIterator;
use itertools::Itertools;

use crate::{ops::interval::Interval, prelude::*, quantity::Zero};

pub type Schedule<V> = Series<V, DateTime<Local>>;

#[must_use]
#[derive(IntoIterator)]
pub struct Series<V, Index>(VecDeque<Slot<V, Index>>);

impl<V, Index> Series<V, Index> {
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

    /// Get the first slot starting index.
    #[must_use]
    pub fn start(&self) -> Option<Index>
    where
        Index: Copy,
    {
        self.0.front().map(|slot| slot.interval.start())
    }

    /// Get the last slot end index, exclusive.
    #[must_use]
    pub fn end(&self) -> Option<Index>
    where
        Index: Copy,
    {
        self.0.back().map(|slot| slot.interval.end())
    }

    /// Get the schedule total duration. Returns zero for empty schedule.
    #[must_use]
    pub fn duration(&self) -> <Index as Sub>::Output
    where
        Index: Copy + Sub,
        <Index as Sub>::Output: Zero,
    {
        self.start().zip(self.end()).map_or(Zero::ZERO, |(start, end)| end - start)
    }

    /// Retrieve the interval and value at the given index.
    ///
    /// Panics outside the bounds.
    pub fn get(&self, index: usize) -> Slot<&V, Index>
    where
        Index: Copy,
    {
        self.0[index].as_ref()
    }

    /// Retrieve the mutable reference at the given index.
    ///
    /// Panics outside the bounds.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> &mut V {
        &mut self.0[index].value
    }

    /// Returns [`true`], if any *matching* slot differs from the corresponding slot in the other schedule.
    ///
    /// Note: non-matched time slots make no difference, the schedules cover for each other.
    pub fn differs_from_by(&self, other: &Self, mut differs: impl FnMut(&V, &V) -> bool) -> bool
    where
        Index: Copy + PartialOrd,
    {
        let mut this = self.iter().peekable();
        let mut other = other.iter().peekable();
        while let (Some(this_slot), Some(other_slot)) = (this.peek(), other.peek()) {
            if this_slot.interval.is_earlier_than(other_slot.interval) {
                this.next();
            } else if other_slot.interval.is_earlier_than(this_slot.interval) {
                other.next();
            } else if !this_slot.interval.contains(other_slot.interval)
                && !other_slot.interval.contains(this_slot.interval)
            {
                // Mutual partial overlap is a significant difference:
                return true;
            } else if differs(this_slot.value, other_slot.value) {
                return true;
            } else {
                // Matched the values, advance:
                this.next();
                other.next();
            }
        }
        false
    }

    /// Construct new schedule by mapping the schedule values.
    pub fn map<T>(&self, mut mapper: impl FnMut(&V) -> T) -> Series<T, Index>
    where
        Index: Copy,
    {
        Series(self.0.iter().map(|slot| slot.map(&mut mapper)).collect())
    }

    /// Construct a new schedule by mapping the values, stopping at the first error.
    pub fn try_map<T>(&self, mut mapper: impl FnMut(&V) -> Result<T>) -> Result<Series<T, Index>>
    where
        Index: Copy,
    {
        self.0
            .iter()
            .map(|slot| Ok(Slot { interval: slot.interval, value: mapper(&slot.value)? }))
            .collect::<Result<_>>()
            .map(Series)
    }

    pub fn zip_eq<T>(self, iterable: impl IntoIterator<Item = T>) -> Series<(V, T), Index>
    where
        Index: PartialEq,
    {
        Series(
            self.0
                .into_iter()
                .zip_eq(iterable)
                .map(|(lhs, rhs)| Slot { interval: lhs.interval, value: (lhs.value, rhs) })
                .collect(),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = Slot<&V, Index>>
    where
        Index: Copy,
    {
        self.0.iter().map(Slot::as_ref)
    }

    pub fn extend(&mut self, other: Self) -> Result
    where
        Index: Copy + PartialEq,
    {
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
        other: impl IntoIterator<Item = (Interval<Index>, V)>,
    ) -> Result
    where
        Index: Copy + Debug + PartialEq,
    {
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

    /// Remove intervals that ended before the given index
    /// and clamp the first remaining interval's start to that index.
    pub fn advance_to(&mut self, index: Index)
    where
        Index: Copy + PartialOrd,
    {
        self.pop_before(index);
        if let Some(first) = self.0.front_mut() {
            first.interval = first.interval.clamp_start_to(index);
        }
    }

    /// Pop schedule slots that ended before the given index.
    fn pop_before(&mut self, index: Index)
    where
        Index: Copy + PartialOrd,
    {
        while self.0.pop_front_if(|slot| slot.interval.end() <= index).is_some() {}
    }
}

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct Slot<V, Index = DateTime<Local>> {
    pub interval: Interval<Index>,

    /// Payload of this schedule slot.
    pub value: V,
}

impl<V, Index: Copy> Slot<V, Index> {
    pub fn map<T>(&self, mapper: impl FnOnce(&V) -> T) -> Slot<T, Index> {
        Slot { interval: self.interval, value: mapper(&self.value) }
    }

    pub const fn as_ref(&self) -> Slot<&V, Index> {
        Slot { interval: self.interval, value: &self.value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_before() {
        let first_interval = Interval::new(1, 2);
        let second_interval = Interval::new(first_interval.end(), 3);

        let mut schedule = Series::new();
        schedule.extend_from_iter([(first_interval, 1), (second_interval, 2)]).unwrap();

        schedule.pop_before(second_interval.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.get(0), Slot { interval: second_interval, value: &2 });
    }

    #[test]
    fn differs_from_by_value_true() {
        let mut lhs = Series::new();
        lhs.extend_from_iter([(Interval::new(1, 2), 12), (Interval::new(2, 3), 23)]).unwrap();

        let mut rhs = Series::new();
        rhs.extend_from_iter([(Interval::new(1, 2), 13), (Interval::new(2, 3), 23)]).unwrap();

        assert!(lhs.differs_from_by(&rhs, |lhs, rhs| lhs != rhs));
    }

    #[test]
    fn differs_from_by_overlap_true() {
        let mut lhs = Series::new();
        lhs.extend_from_iter([(Interval::new(1, 3), 13)]).unwrap();

        let mut rhs = Series::new();
        rhs.extend_from_iter([(Interval::new(2, 4), 24)]).unwrap();

        assert!(lhs.differs_from_by(&rhs, |_, _| false));
    }

    #[test]
    fn differs_from_by_unmatched_at_start_false() {
        let mut lhs = Series::new();
        lhs.extend_from_iter([(Interval::new(2, 3), 23)]).unwrap();

        let mut rhs = Series::new();
        rhs.extend_from_iter([(Interval::new(1, 2), 12), (Interval::new(2, 3), 23)]).unwrap();

        assert!(!lhs.differs_from_by(&rhs, |lhs, rhs| lhs != rhs));
    }

    #[test]
    fn differs_from_by_unmatched_at_end_false() {
        let mut lhs = Series::new();
        lhs.extend_from_iter([(Interval::new(1, 2), 12), (Interval::new(2, 3), 23)]).unwrap();

        let mut rhs = Series::new();
        rhs.extend_from_iter([(Interval::new(1, 2), 12)]).unwrap();

        assert!(!lhs.differs_from_by(&rhs, |lhs, rhs| lhs != rhs));
    }
}
