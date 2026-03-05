use itertools::Itertools;

use crate::ops::RangeExclusive;

/// Set of non-intersecting unique intervals.
pub struct RangeSet<T>(Vec<RangeExclusive<T>>);

impl<T: Copy + Ord> FromIterator<RangeExclusive<T>> for RangeSet<T> {
    fn from_iter<I: IntoIterator<Item = RangeExclusive<T>>>(iter: I) -> Self {
        Self(iter.into_iter().sorted_by_key(|range| (range.start, range.end)).dedup().collect())
    }
}

impl<T> RangeSet<T> {
    pub fn find_containing(&self, inner: RangeExclusive<T>) -> Option<RangeExclusive<T>>
    where
        T: Copy + Ord,
    {
        let partition_point = self.0.partition_point(|interval| inner.start >= interval.start);
        self.0[..partition_point].last().filter(|interval| interval.end >= inner.end).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_positive() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(1..3), RangeExclusive::from_std(3..5)]
            .into_iter()
            .collect();
        assert_eq!(
            set.find_containing(RangeExclusive::from_std(1..2)),
            Some(RangeExclusive::from_std(1..3))
        );
    }

    #[test]
    fn contains_negative() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(1..3), RangeExclusive::from_std(3..5)]
            .into_iter()
            .collect();
        assert!(set.find_containing(RangeExclusive::from_std(2..4)).is_none());
    }

    #[test]
    fn exact_match() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(1..3), RangeExclusive::from_std(3..5)]
            .into_iter()
            .collect();
        assert_eq!(
            set.find_containing(RangeExclusive::from_std(1..3)),
            Some(RangeExclusive::from_std(1..3)),
        );
    }

    #[test]
    fn falls_in_gap() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(0..2), RangeExclusive::from_std(3..5)]
            .into_iter()
            .collect();
        assert!(set.find_containing(RangeExclusive::from_std(2..3)).is_none());
    }

    #[test]
    fn query_larger_than_stored() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(2..3)].into_iter().collect();
        assert!(set.find_containing(RangeExclusive::from_std(1..5)).is_none());
    }

    #[test]
    fn touching_neighbor_not_returned() {
        let set: RangeSet<_> = vec![RangeExclusive::from_std(1..3), RangeExclusive::from_std(3..5)]
            .into_iter()
            .collect();
        assert_eq!(
            set.find_containing(RangeExclusive::from_std(1..3)),
            Some(RangeExclusive::from_std(1..3)),
        );
    }

    #[test]
    fn empty_set() {
        let set: RangeSet<i32> = vec![].into_iter().collect();
        assert!(set.find_containing(RangeExclusive::from_std(0..1)).is_none());
    }
}
