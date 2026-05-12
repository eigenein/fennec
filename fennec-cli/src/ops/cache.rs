use std::time::{Duration, Instant};

use crate::{ops::Cache, prelude::*};

impl<V> Cache<V> {
    pub const fn new(time_to_live: Duration) -> Self {
        Self { time_to_live, entry: None }
    }

    pub async fn get_or_insert_with(
        &mut self,
        init: impl Future<Output = Result<V>>,
    ) -> Result<&V> {
        if !matches!(
            &self.entry,
            Some(entry) if entry.timestamp.elapsed() <= self.time_to_live
        ) {
            self.entry = Some(Entry::now(init.await?));
        }
        Ok(&self.entry.as_ref().unwrap().value)
    }
}

/// Timestamped cache entry.
#[must_use]
pub struct Entry<T> {
    pub timestamp: Instant,
    pub value: T,
}

impl<T> Entry<T> {
    pub fn now(value: T) -> Self {
        Self { timestamp: Instant::now(), value }
    }
}
