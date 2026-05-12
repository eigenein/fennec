pub mod cache;
mod integrator;
pub mod range;
pub mod schedule;

use std::time::Duration;

use chrono::TimeZone;

pub use self::integrator::{BucketIntegrator, BucketMean, Integrator};
use self::schedule::Interval;
use crate::prelude::*;

/// Simple one-value time-to-live cache.
#[must_use]
pub struct Cache<V> {
    time_to_live: Duration,
    entry: Option<cache::Entry<V>>,
}

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
            self.entry = Some(cache::Entry::now(init.await?));
        }
        Ok(&self.entry.as_ref().unwrap().value)
    }
}

/// Non-empty, ordered and continuous schedule.
pub struct Schedule<Tz: TimeZone, V>(Box<[(Interval<Tz>, V)]>);
