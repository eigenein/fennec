use anyhow::{Error, ensure};
use chrono::{DateTime, Local, TimeZone};

use crate::{ops::range, prelude::*, quantity::time::Hours};

/// Half-open time interval.
pub type Interval<Tz = Local> = range::Exclusive<DateTime<Tz>>;

impl<Tz: TimeZone> Interval<Tz> {
    /// Interval duration.
    pub fn duration(self) -> Hours {
        (self.end - self.start).into()
    }
}

/// Non-empty, ordered and continuous schedule.
pub struct Schedule<Tz: TimeZone, V>(Box<[(Interval<Tz>, V)]>);

impl<Tz: TimeZone, V> TryFrom<Box<[(Interval<Tz>, V)]>> for Schedule<Tz, V> {
    type Error = Error;

    fn try_from(inner: Box<[(Interval<Tz>, V)]>) -> Result<Self> {
        ensure!(!inner.is_empty(), "the schedule is empty");
        for [(previous, _), (next, _)] in inner.array_windows() {
            ensure!(previous.end == next.start, "item `{next:?}` cannot follow `{previous:?}`");
        }
        Ok(Self(inner))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

    /// Verify valid schedule conversion.
    #[test]
    fn try_from_ok() {
        let first = Local.with_ymd_and_hms(2026, 5, 12, 20, 10, 0).unwrap();
        let second = Local.with_ymd_and_hms(2026, 5, 12, 20, 20, 0).unwrap();
        let third = Local.with_ymd_and_hms(2026, 5, 12, 20, 30, 0).unwrap();

        let inner = Box::from([
            (Interval { start: first, end: second }, 1),
            (Interval { start: second, end: third }, 2),
        ]);
        assert!(Schedule::try_from(inner).is_ok());
    }

    /// Verify that empty schedule is invalid.
    #[test]
    fn try_from_empty() {
        assert!(Schedule::<Local, ()>::try_from(Box::from([])).is_err());
    }

    /// Verify that schedule with a gap is invalid.
    #[test]
    fn try_from_broken() {
        let entry_1 = Local.with_ymd_and_hms(2026, 5, 12, 20, 10, 0).unwrap();
        let entry_2 = Local.with_ymd_and_hms(2026, 5, 12, 20, 20, 0).unwrap();
        let entry_3 = Local.with_ymd_and_hms(2026, 5, 12, 20, 30, 0).unwrap();
        let entry_4 = Local.with_ymd_and_hms(2026, 5, 12, 20, 40, 0).unwrap();

        let inner = Box::from([
            (Interval { start: entry_1, end: entry_2 }, 1),
            (Interval { start: entry_3, end: entry_4 }, 2),
        ]);
        assert!(Schedule::try_from(inner).is_err());
    }
}
