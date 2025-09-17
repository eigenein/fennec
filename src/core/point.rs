use chrono::{DateTime, Local};

use crate::prelude::*;

/// A time series point.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    derive_more::Constructor,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Point<V> {
    pub time: DateTime<Local>,
    pub value: V,
}

impl<V> Point<V> {
    pub fn try_zip<'l, 'r, R>(&'l self, rhs: &'r Point<R>) -> Result<Point<(&'l V, &'r R)>> {
        ensure!(self.time == rhs.time);
        Ok(Point::new(self.time, (&self.value, &rhs.value)))
    }

    pub fn map<T>(self, f: fn(V) -> T) -> Point<T> {
        Point::new(self.time, f(self.value))
    }
}
