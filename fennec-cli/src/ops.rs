pub mod cache;
mod integrator;
pub mod range;
pub mod schedule;

use chrono::TimeZone;

pub use self::integrator::{BucketIntegrator, BucketMean, Integrator};
use self::schedule::Interval;

/// Non-empty, ordered and continuous schedule.
pub struct Schedule<Tz: TimeZone, V>(Box<[(Interval<Tz>, V)]>);
