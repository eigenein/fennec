use chrono::{DateTime, Local, TimeZone};

use crate::quantity::time::Hours;

/// Half-open time interval.
#[must_use]
#[derive(Copy, Clone)]
pub struct Interval<Tz = Local>
where
    Tz: TimeZone,
    DateTime<Tz>: Copy,
{
    start: DateTime<Tz>,
    end: DateTime<Tz>,
}

impl<Tz> Interval<Tz>
where
    Tz: TimeZone,
    DateTime<Tz>: Copy,
{
    pub fn new(start: DateTime<Tz>, end: DateTime<Tz>) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    pub const fn start(self) -> DateTime<Tz>
    where
        Self: Copy,
    {
        self.start
    }

    pub const fn end(self) -> DateTime<Tz>
    where
        Self: Copy,
    {
        self.end
    }

    /// Restrict the interval start to the specified timestamp.
    pub fn clamp_start(mut self, to: DateTime<Tz>) -> Self {
        if to > self.end {
            self.start = self.end;
        } else if to > self.start {
            self.start = to;
        }
        self
    }

    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_start() {
        let start = Local.with_ymd_and_hms(2026, 5, 15, 14, 40, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 5, 15, 14, 50, 0).unwrap();
        let interval = Interval { start, end };

        // Target before the interval does not change the interval:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 30, 0).unwrap();
        assert_eq!(interval.clamp_start(to).start, start);

        // Target within the interval clamps to the target:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 45, 0).unwrap();
        assert_eq!(interval.clamp_start(to).start, to);

        // Target after the interval clamps to the end:
        let to = Local.with_ymd_and_hms(2026, 5, 15, 14, 55, 0).unwrap();
        assert_eq!(interval.clamp_start(to).start, end);
    }
}
