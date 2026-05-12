use std::time::{Duration, Instant};

use crate::prelude::*;

/// Simple one-value time-to-live cache.
pub struct Cache<T> {
    time_to_live: Duration,
    entry: Option<Entry<T>>,
}

impl<T> Cache<T> {
    pub const fn new(time_to_live: Duration) -> Self {
        Self { time_to_live, entry: None }
    }

    pub async fn get_with(&mut self, init: impl Future<Output = Result<T>>) -> Result<&T> {
        if !matches!(
            &self.entry,
            Some(entry) if entry.timestamp.elapsed() <= self.time_to_live
        ) {
            self.entry = Some(Entry::now(init.await?));
        }
        Ok(&self.entry.as_ref().unwrap().value)
    }
}

struct Entry<T> {
    timestamp: Instant,
    value: T,
}

impl<T> Entry<T> {
    fn now(value: T) -> Self {
        Self { timestamp: Instant::now(), value }
    }
}
