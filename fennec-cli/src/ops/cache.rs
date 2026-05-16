use std::time::{Duration, Instant};

use crate::prelude::*;

#[must_use]
pub struct Ttl<V> {
    /// Time-to-live duration.
    duration: Duration,

    /// Cached value along with the deadline.
    slot: Option<(Instant, V)>,
}

impl<V> Ttl<V> {
    pub const fn new(duration: Duration) -> Self {
        Self { duration, slot: None }
    }

    pub async fn get_or_insert_with(
        &mut self,
        init: impl Future<Output = Result<V>>,
    ) -> Result<&V> {
        if self.slot.as_ref().is_none_or(|(deadline, _)| Instant::now() > *deadline) {
            self.slot = Some((Instant::now() + self.duration, init.await?));
        }
        Ok(&self.slot.as_ref().unwrap().1)
    }
}
