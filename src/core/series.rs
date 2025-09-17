use std::{iter::Map, slice::Iter};

use chrono::{DateTime, Local};

use crate::{core::Point, prelude::*};

#[derive(
    Clone,
    derive_more::Index,
    derive_more::IndexMut,
    derive_more::IntoIterator,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Series<V>(Vec<Point<V>>);

impl<V> FromIterator<Point<V>> for Series<V> {
    fn from_iter<I: IntoIterator<Item = Point<V>>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'v, V> IntoIterator for &'v Series<V> {
    type Item = Point<&'v V>;

    type IntoIter = Map<Iter<'v, Point<V>>, fn(&Point<V>) -> Point<&V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(Point::as_ref)
    }
}

impl<V> Series<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = Point<&V>> {
        self.into_iter()
    }

    /// Push the point.
    pub fn push(&mut self, time: DateTime<Local>, value: V) {
        self.0.push(Point { time, value });
    }

    /// Map each point value.
    pub fn map<T>(self, f: fn(V) -> T) -> impl IntoIterator<Item = Point<T>> {
        self.0.into_iter().map(move |point| point.map(f))
    }

    /// Zip the time series by the point timestamps.
    ///
    /// `try_zip()` returns an error when the timestamps do not match.
    pub fn try_zip<R>(
        &self,
        rhs: impl IntoIterator<Item = Point<R>>,
    ) -> impl Iterator<Item = Result<Point<(&V, R)>>> {
        self.0.iter().zip(rhs).map(|(lhs, rhs)| lhs.try_zip(rhs))
    }
}
