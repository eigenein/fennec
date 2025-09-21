mod serde;

use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use chrono::{DateTime, Local};
use itertools::{EitherOrBoth, Itertools};

use crate::{core::working_mode::WorkingMode, prelude::*};

/// Series of values sorted by index.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq, derive_more::IntoIterator)]
pub struct Series<V, I = DateTime<Local>>(#[into_iterator(owned, ref)] Vec<(I, V)>);

impl<V, I> Default for Series<V, I> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<V, I: Ord> FromIterator<(I, V)> for Series<V, I> {
    fn from_iter<Iter: IntoIterator<Item = (I, V)>>(iter: Iter) -> Self {
        let mut this = Self(iter.into_iter().collect());
        // FIXME: `try_collect` isn't stable, so for now, just sort it to ensure the ordering:
        this.0.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        this
    }
}

impl<V, I> Index<usize> for Series<V, I> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index].1
    }
}

impl<V, I> IndexMut<usize> for Series<V, I> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index].1
    }
}

impl<V, I> Series<V, I> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &(I, V)> {
        self.into_iter()
    }
}

impl<V, I: Ord> Series<V, I> {
    pub fn try_extend(&mut self, other: impl IntoIterator<Item = (I, V)>) -> Result {
        self.0.extend(other);
        self.assert_sorted()
    }

    /// Attempt to push a point.
    ///
    /// The function fails if the point violates the ordering.
    pub fn try_push(&mut self, index: I, value: V) -> Result {
        ensure!(self.0.last().is_none_or(|(last_index, _)| last_index < &index));
        self.0.push((index, value));
        Ok(())
    }

    /// Zip the series by the indices.
    ///
    /// - Matched indices are zipped together and the right-hand side value is mapped.
    /// - Missing indices on the left side are skipped.
    /// - Missing indices on the right side are replaced with the `default`.
    pub fn zip_right_or<R, T: Copy>(
        &self,
        rhs: &Series<R, I>,
        map: fn(&R) -> T,
        default: T,
    ) -> impl Iterator<Item = (&I, (&V, T))> {
        self.0.iter().merge_join_by(&rhs.0, |(lhs, _), (rhs, _)| lhs.cmp(rhs)).filter_map(
            move |pair| match pair {
                EitherOrBoth::Both((left_index, left_value), (_, right_value)) => {
                    Some((left_index, (left_value, map(right_value))))
                }
                EitherOrBoth::Left((left_index, left_value)) => {
                    Some((left_index, (left_value, default)))
                }
                EitherOrBoth::Right(_) => None,
            },
        )
    }

    fn assert_sorted(&self) -> Result {
        ensure!(self.0.is_sorted_by_key(|(index, _)| index));
        Ok(())
    }
}

impl<V, I: Debug + Ord> Series<V, I> {
    /// Zip the series by the indices.
    ///
    /// It returns an error when the indices do not match.
    pub fn try_zip_exactly<'l, 'r, R>(
        &'l self,
        rhs: &'r Series<R, I>,
    ) -> impl Iterator<Item = Result<(&'l I, (&'l V, &'r R))>> {
        self.0.iter().merge_join_by(&rhs.0, |(lhs, _), (rhs, _)| lhs.cmp(rhs)).map(
            |pair| match pair {
                EitherOrBoth::Both((left_index, left_value), (_, right_value)) => {
                    Ok((left_index, (left_value, right_value)))
                }
                EitherOrBoth::Left((index, _)) | EitherOrBoth::Right((index, _)) => {
                    bail!("non-matching index: `{index:?}`");
                }
            },
        )
    }
}

impl<I: Copy> Series<WorkingMode, I> {
    const MODES: [WorkingMode; 4] = [
        WorkingMode::Idle,
        WorkingMode::Balancing,
        WorkingMode::Charging,
        WorkingMode::Discharging,
    ];

    pub fn mutate(&mut self) -> (Mutation<WorkingMode>, Mutation<WorkingMode>) {
        let len = self.0.len();
        assert!(len >= 2);

        let index_1 = fastrand::usize(0..(len - 1));
        let mutation_1 = Mutation { index: index_1, old_value: self[index_1] };

        let index_2 = fastrand::usize(index_1..len);
        let mutation_2 = Mutation { index: index_2, old_value: self[index_2] };

        (self[index_1], self[index_2]) = loop {
            let new_1 = fastrand::choice(Self::MODES).unwrap();
            let new_2 = fastrand::choice(Self::MODES).unwrap();
            if (new_1, new_2) != (self[index_1], self[index_2]) {
                break (new_1, new_2);
            }
        };

        (mutation_1, mutation_2)
    }
}

pub struct Mutation<V> {
    pub index: usize,
    pub old_value: V,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_zip_exactly_ok() -> Result {
        let lhs = Series::from_iter([(42, 1)]);
        let rhs = Series::from_iter([(42, 2)]);
        assert_eq!(lhs.try_zip_exactly(&rhs).next().unwrap()?, (&42, (&1, &2)));
        Ok(())
    }

    #[test]
    fn test_try_zip_exactly_error() {
        let lhs = Series::from_iter([(42, 1)]);
        let rhs = Series::from_iter([(43, 2)]);
        assert!(lhs.try_zip_exactly(&rhs).next().unwrap().is_err());
    }

    #[test]
    fn test_zip_right_or() {
        let lhs = Series::from_iter([(42, 2), (43, 4)]);
        let rhs = Series::from_iter([(41, 1), (42, 3)]);
        assert_eq!(
            lhs.zip_right_or(&rhs, |rhs| Some(*rhs), None).collect_vec(),
            [(&42, (&2, Some(3))), (&43, (&4, None))]
        );
    }

    #[test]
    fn test_mutate() -> Result {
        let mut series = Series::from_iter([
            (1, WorkingMode::default()),
            (2, WorkingMode::default()),
            (3, WorkingMode::default()),
        ]);
        let original = series.clone();

        let (mutation_1, mutation_2) = series.mutate();
        assert_ne!(series, original, "the mutated series must differ from the original");

        series[mutation_1.index] = mutation_1.old_value;
        series[mutation_2.index] = mutation_2.old_value;
        assert_eq!(series, original, "the restored series must equal to the original");
        Ok(())
    }

    #[test]
    fn test_extend_ok() -> Result {
        Series::from_iter([(1, 1)]).try_extend(Series::from_iter([(2, 2)]))?;
        Ok(())
    }

    #[test]
    fn test_extend_error() {
        assert!(Series::from_iter([(3, 3)]).try_extend(Series::from_iter([(2, 2)])).is_err());
    }
}
