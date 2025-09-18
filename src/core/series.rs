use std::ops::{Index, IndexMut};

use chrono::{DateTime, Local};

use crate::{core::point::Point, prelude::*};

#[derive(Clone, derive_more::IntoIterator, serde::Deserialize, serde::Serialize)]
pub struct Series<V>(#[into_iterator(owned, ref)] Vec<Point<V>>);

impl<V> FromIterator<Point<V>> for Series<V> {
    fn from_iter<I: IntoIterator<Item = Point<V>>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V> Index<usize> for Series<V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index].value
    }
}

impl<V> IndexMut<usize> for Series<V> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index].value
    }
}

impl<V> Series<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub const fn len(&self) -> usize {
        self.0.len()
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
    pub fn try_zip<'l, 'r, R>(
        &'l self,
        rhs: &'r Series<R>,
    ) -> impl Iterator<Item = Result<Point<(&'l V, &'r R)>>> {
        self.0.iter().zip(&rhs.0).map(|(lhs, rhs)| lhs.try_zip(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_zip_ok() -> Result {
        let time = Local::now();
        let lhs = Series::from_iter([Point::new(time, 1)]);
        let rhs = Series::from_iter([Point::new(time, 2)]);
        assert_eq!(lhs.try_zip(&rhs).next().unwrap()?, Point::new(time, (&1, &2)));
        Ok(())
    }
}
