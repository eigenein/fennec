use chrono::{DateTime, Local};

use crate::{core::point::Point, prelude::*};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Series<V>(Vec<Point<V>>);

impl<V> FromIterator<Point<V>> for Series<V> {
    fn from_iter<I: IntoIterator<Item = Point<V>>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V> Series<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (DateTime<Local>, &V)> {
        self.0.iter().map(|point| (point.time, &point.value))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (DateTime<Local>, &mut V)> {
        self.0.iter_mut().map(|point| (point.time, &mut point.value))
    }

    /// Push the point.
    pub fn push(&mut self, time: DateTime<Local>, value: V) {
        self.0.push(Point { time, value });
    }

    /// Map each point value.
    pub fn map<T>(self, f: fn(V) -> T) -> impl IntoIterator<Item = (DateTime<Local>, T)> {
        self.0.into_iter().map(move |point| (point.time, f(point.value)))
    }

    /// Zip the time series by the point timestamps.
    ///
    /// `try_zip()` returns an error when the timestamps do not match.
    pub fn try_zip<'l, 'r, R>(
        &'l self,
        rhs: &'r Series<R>,
    ) -> impl Iterator<Item = Result<(DateTime<Local>, &'l V, &'r R)>> {
        // FIXME: should call `zip_longest` and verify no leftovers.
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
        assert_eq!(lhs.try_zip(&rhs).next().unwrap()?, (time, &1, &2));
        Ok(())
    }
}
