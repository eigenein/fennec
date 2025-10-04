mod consumption;
pub mod stats;

use std::{collections::BTreeMap, fmt::Debug};

use chrono::{DateTime, Local};
use serde_with::serde_as;

/// Series of values sorted by index.
///
/// Technically, I could implement it using a [`Vec`] while carefully maintaining the invariant,
/// but [`BTreeMap`] makes it much easier without a big performance penalty.
///
/// TODO: I guess, I should make specific traits over `IntoIterator::<Item = (I, V)>` to support any container
///       and avoid the extra `collect()` calls (`Differentiate`, `ResampleHourly`, and `AverageHourly`).
#[must_use]
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, derive_more::IntoIterator, serde::Serialize)]
pub struct Series<V, I: Ord = DateTime<Local>>(
    #[into_iterator(owned, ref)]
    #[serde_as(as = "serde_with::Seq<(_, _)>")]
    #[serde(bound(serialize = "I: serde::Serialize, V: serde::Serialize"))]
    BTreeMap<I, V>,
);

impl<V, I: Ord> Default for Series<V, I> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<V, I: Ord> FromIterator<(I, V)> for Series<V, I> {
    fn from_iter<Iter: IntoIterator<Item = (I, V)>>(iter: Iter) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V, I: Ord> Series<V, I> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&I, &V)> {
        self.into_iter()
    }
}
