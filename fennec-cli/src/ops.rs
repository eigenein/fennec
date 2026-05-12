pub mod cache;
mod integrator;
mod interval;
pub mod range;

use std::time::Duration;

pub use self::{
    integrator::{BucketIntegrator, BucketMean, Integrator},
    interval::Interval,
};
use crate::prelude::*;

/// Simple one-value time-to-live cache.
#[must_use]
pub struct Cache<T> {
    time_to_live: Duration,
    entry: Option<cache::Entry<T>>,
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
            self.entry = Some(cache::Entry::now(init.await?));
        }
        Ok(&self.entry.as_ref().unwrap().value)
    }
}
