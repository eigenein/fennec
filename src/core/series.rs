use std::{collections::BTreeMap, fmt::Debug};

use chrono::{DateTime, Local};
use itertools::{EitherOrBoth, Itertools};

use crate::prelude::*;

#[derive(Clone, serde::Deserialize, serde::Serialize, derive_more::IntoIterator)]
pub struct Series<V, I: Ord = DateTime<Local>>(#[into_iterator(owned, ref)] BTreeMap<I, V>);

impl<V, I: Ord> Default for Series<V, I> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<V, I: Ord> FromIterator<(I, V)> for Series<V, I> {
    fn from_iter<Iter: IntoIterator<Item = (I, V)>>(iter: Iter) -> Self {
        Self(BTreeMap::from_iter(iter))
    }
}

impl<V, I: Ord> Series<V, I> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&I, &V)> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&I, &mut V)> {
        self.0.iter_mut()
    }

    pub fn insert(&mut self, index: I, value: V) {
        self.0.insert(index, value);
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = (I, V)>) {
        self.0.extend(other);
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

    /// Zip the series by the indices.
    ///
    /// Missing indices on the left side are skipped,
    /// and missing indices on the right side are replaced with the `default`.
    pub fn zip_right_or<'l, 'r, R>(
        &'l self,
        rhs: &'r Series<R, I>,
        default: &'r R,
    ) -> impl Iterator<Item = (&'l I, (&'l V, &'r R))> {
        self.0.iter().merge_join_by(&rhs.0, |(lhs, _), (rhs, _)| lhs.cmp(rhs)).filter_map(
            move |pair| match pair {
                EitherOrBoth::Both((left_index, left_value), (_, right_value)) => {
                    Some((left_index, (left_value, right_value)))
                }
                EitherOrBoth::Left((left_index, left_value)) => {
                    Some((left_index, (left_value, default)))
                }
                EitherOrBoth::Right(_) => None,
            },
        )
    }
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
        assert_eq!(lhs.zip_right_or(&rhs, &5).collect_vec(), [(&42, (&2, &3)), (&43, (&4, &5))]);
    }
}
