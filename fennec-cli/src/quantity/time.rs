use chrono::TimeDelta;

quantity!(Hours, via: f64, suffix: "h", precision: 2);

impl From<TimeDelta> for Hours {
    fn from(time_delta: TimeDelta) -> Self {
        Self(time_delta.as_seconds_f64() / 3600.0)
    }
}

impl Hours {
    /// Convert the duration to days.
    pub const fn days(self) -> f64 {
        self.0 / 24.0
    }
}
