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
    pub fn start_index(&self) -> Option<Index>
    where
        Index: Copy,
    {
        self.0.front().map(|slot| slot.interval.start())
    }

    /// Get the last slot end index, exclusive.
    #[must_use]
    pub fn end_index(&self) -> Option<Index>
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
        self.start_index().zip(self.end_index()).map_or(Zero::ZERO, |(start, end)| end - start)
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
            self.end_index().zip(other.start_index()).is_none_or(|(end, start)| end == start),
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
            let current_end = self.end_index();
            ensure!(
                current_end.is_none_or(|current_end| current_end == interval.start()),
                "trying to push `{interval:?}` on top of `{current_end:?}`",
            );
            self.0.push_back(Slot { interval, value });
        }
        Ok(())
    }

    /// Remove slots that ended at or before the given index
    /// and clamp the first remaining interval's start to that index.
    ///
    /// Returns number of removed slots.
    pub fn advance_to(&mut self, index: Index) -> usize
    where
        Index: Copy + PartialOrd,
    {
        let n_removed = self.remove_before(index);
        if let Some(first_slot) = self.0.front_mut() {
            first_slot.interval = first_slot.interval.clamp_start_to(index);
        }
        n_removed
    }

    /// Remove schedule slots that ended at or before the given index.
    ///
    /// Returns the number of removed slots.
    fn remove_before(&mut self, index: Index) -> usize
    where
        Index: Copy + PartialOrd,
    {
        std::iter::from_fn(|| self.0.pop_front_if(|slot| slot.interval.end() <= index)).count()
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
    fn remove_before() {
        let first_interval = Interval::new(1, 2);
        let second_interval = Interval::new(first_interval.end(), 3);

        let mut schedule = Series::new();
        schedule.extend_from_iter([(first_interval, 1), (second_interval, 2)]).unwrap();

        schedule.remove_before(second_interval.start());
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule.get(0), Slot { interval: second_interval, value: &2 });
    }
}
