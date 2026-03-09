use chrono::{NaiveTime, TimeDelta};

use crate::quantity::energy::WattHours;

pub trait Quantum<V>: Sized {
    /// Project the value into a bucket index.
    #[must_use]
    fn index(self, value: V) -> Option<usize>;

    /// Un-project the bucket index into the value that represents the middle of the bucket.
    #[must_use]
    fn midpoint(self, index: usize) -> Option<V>;
}

impl Quantum<NaiveTime> for TimeDelta {
    fn index(self, value: NaiveTime) -> Option<usize> {
        let nanos_since_midnight = (value - NaiveTime::MIN).num_nanoseconds()?;
        let quantum_nanos = self.num_nanoseconds()?;
        usize::try_from(nanos_since_midnight.checked_div(quantum_nanos)?).ok()
    }

    fn midpoint(self, index: usize) -> Option<NaiveTime> {
        let quantum_nanoseconds = self.num_nanoseconds()?;
        let duration_nanos = i64::try_from(index)
            .ok()?
            .checked_mul(quantum_nanoseconds)?
            .checked_add(quantum_nanoseconds / 2)?;
        let (naive_time, days) =
            NaiveTime::MIN.overflowing_add_signed(Self::nanoseconds(duration_nanos));
        (days == 0).then_some(naive_time)
    }
}

impl Quantum<Self> for WattHours {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn index(self, value: Self) -> Option<usize> {
        let index = (value / self).floor();
        (index >= 0.0).then_some(index as usize)
    }

    #[expect(clippy::cast_precision_loss)]
    fn midpoint(self, index: usize) -> Option<Self> {
        Some(self * (index as f64) + self / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_max_naive_time() {
        let secs = 23 * 3600 + 59 * 60 + 59;
        let naive_time = NaiveTime::from_num_seconds_from_midnight_opt(secs, 999_999_999).unwrap();
        assert_eq!(TimeDelta::minutes(15).index(naive_time).unwrap(), 95);
    }

    #[test]
    fn dequantize_naive_time() {
        assert_eq!(
            TimeDelta::minutes(15).midpoint(95).unwrap(),
            NaiveTime::from_hms_opt(23, 52, 30).unwrap(),
        );
    }

    #[test]
    fn quantize_energy() {
        assert_eq!(WattHours(1.0).index(WattHours(2.999)).unwrap(), 2);
    }

    #[test]
    fn dequantize_energy() {
        assert_eq!(WattHours(1.0).midpoint(2).unwrap(), WattHours(2.5));
    }
}
