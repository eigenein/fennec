use std::f64::consts::TAU;

use chrono::{DateTime, Datelike, Local, NaiveTime, TimeDelta, Timelike};

use crate::quantity::Quantity;

pub type Radians<V = f64> = Quantity<V, 0, 0, 0, 0>;

impl Radians {
    /// Phase of the daily cycle.
    ///
    /// For example, 12:00 is π, and 23:59:59 is just below 2π.
    pub fn daily_phase_at(time: NaiveTime) -> Self {
        let nanos = u64::from(time.num_seconds_from_midnight()) * 1_000_000_000
            + u64::from(time.nanosecond());

        #[expect(clippy::cast_precision_loss)]
        Self::daily_phase_shift_from_secs(nanos as f64 / 1_000_000_000.0)
    }

    /// Phase of the weekly cycle.
    ///
    /// For example, Monday 00:00 is `0.0`, and Sunday 23:59:59 is just below 2π.
    ///
    /// Note: at Sunday 23:59:59 during a represented leap second,
    /// the result can reach or very slightly exceed 2π.
    pub fn weekly_phase_at(timestamp: DateTime<Local>) -> Self {
        const NANOS_PER_SECOND: u64 = 1_000_000_000;
        const NANOS_PER_DAY: u64 = 86_400 * NANOS_PER_SECOND;
        const NANOS_PER_WEEK: f64 = 604_800_000_000_000.0;

        let nanos = u64::from(timestamp.weekday().num_days_from_monday()) * NANOS_PER_DAY
            + u64::from(timestamp.num_seconds_from_midnight()) * NANOS_PER_SECOND
            + u64::from(timestamp.nanosecond());

        #[expect(clippy::cast_precision_loss)]
        Self(nanos as f64 / NANOS_PER_WEEK * TAU)
    }

    /// Phase shift in the daily cycle corresponding to the given duration.
    ///
    /// For example, 6 hours is ½π.
    pub fn daily_phase_shift_of(time_delta: TimeDelta) -> Self {
        Self::daily_phase_shift_from_secs(time_delta.as_seconds_f64())
    }

    /// Phase shift in the daily cycle corresponding to the given duration in seconds.
    ///
    /// For example, 43200 seconds (half a day) is π.
    pub const fn daily_phase_shift_from_secs(seconds: f64) -> Self {
        const SECONDS_PER_DAY: f64 = 86_400.0;
        Self(seconds / SECONDS_PER_DAY * TAU)
    }
}
