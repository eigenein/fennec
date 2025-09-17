use chrono::{DateTime, Local};

use crate::prelude::*;

/// A time series point.
#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Point<V> {
    pub time: DateTime<Local>,
    pub value: V,
}

impl<V> Point<V> {
    /// Convert from `&Point<V>` to `Point<&V>`.
    pub const fn as_ref(&self) -> Point<&V> {
        Point { time: self.time, value: &self.value }
    }

    pub fn try_zip<R>(&self, rhs: Point<R>) -> Result<Point<(&V, R)>> {
        ensure!(self.time == rhs.time);
        Ok(Point { time: self.time, value: (&self.value, rhs.value) })
    }

    pub fn map<T>(self, f: fn(V) -> T) -> Point<T> {
        Point { time: self.time, value: f(self.value) }
    }
}
