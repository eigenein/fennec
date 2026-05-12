pub mod cache;
mod integrator;
pub mod range;
pub mod schedule;

use std::time::Duration;

use chrono::TimeZone;

pub use self::integrator::{BucketIntegrator, BucketMean, Integrator};
use self::schedule::Interval;

/// Simple one-value time-to-live cache.
#[must_use]
pub struct Cache<V> {
    time_to_live: Duration,
    entry: Option<cache::Entry<V>>,
}

/// Non-empty, ordered and continuous schedule.
pub struct Schedule<Tz: TimeZone, V>(Box<[(Interval<Tz>, V)]>);
