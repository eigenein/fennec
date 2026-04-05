#![allow(unused_imports)]

use std::time::Duration;

pub use anyhow::{Context, Error, anyhow, bail, ensure};
pub use tracing::{Level, debug, error, info, instrument, trace, warn};

pub type Result<T = (), E = Error> = anyhow::Result<T, E>;

pub fn log_error(error: &Error, sleep_duration: Duration) {
    warn!(retry_in = ?sleep_duration, "{error:#}");
}
