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
    /// Zip the time series by the point timestamps.
    ///
    /// `try_zip_exactly()` returns an error when the timestamps do not match.
    pub fn try_zip_exactly<'l, 'r, R>(
        &'l self,
        rhs: &'r Series<R, I>,
    ) -> impl Iterator<Item = Result<(&'l I, &'l V, &'r R)>> {
        self.0.iter().zip_longest(&rhs.0).map(|pair| match pair {
            EitherOrBoth::Both((left_index, left_value), (right_index, right_value)) => {
                if left_index == right_index {
                    Ok((left_index, left_value, right_value))
                } else {
                    bail!("indexes do not match: `{left_index:?}` vs `{right_index:?}`");
                }
            }
            _ => bail!("the series lengths do not match"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_zip_ok() -> Result {
        let lhs = Series::from_iter([(42, 1)]);
        let rhs = Series::from_iter([(42, 2)]);
        assert_eq!(lhs.try_zip_exactly(&rhs).next().unwrap()?, (&42, &1, &2));
        Ok(())
    }
}
