use chrono::TimeDelta;

quantity!(Hours, via: f64, suffix: "h", precision: 1);

impl From<TimeDelta> for Hours {
    fn from(time_delta: TimeDelta) -> Self {
        Self(time_delta.as_seconds_f64() / 3600.0)
    }
}
