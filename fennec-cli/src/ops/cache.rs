use std::time::{Duration, Instant};

use crate::prelude::*;

/// Cache that persist till the deadline.
#[must_use]
pub struct Deadline<V>(Option<(Instant, V)>);

impl<V> Deadline<V> {
    pub async fn get_or_insert_with(
        &mut self,
        init: impl Future<Output = Result<(Instant, V)>>,
    ) -> Result<&V> {
        if !matches!(&self.0, Some((deadline, _)) if Instant::now() <= *deadline) {
            self.0 = Some(init.await?);
        }
        Ok(&self.0.as_ref().unwrap().1)
    }
}

#[must_use]
pub struct Ttl<V> {
    duration: Duration,
    inner: Deadline<V>,
}

impl<V> Ttl<V> {
    pub const fn new(duration: Duration) -> Self {
        Self { duration, inner: Deadline(None) }
    }

    pub async fn get_or_insert_with(
        &mut self,
        init: impl Future<Output = Result<V>>,
    ) -> Result<&V> {
        self.inner
            .get_or_insert_with(async {
                let deadline = Instant::now() + self.duration;
                Ok((deadline, init.await?))
            })
            .await
    }
}
