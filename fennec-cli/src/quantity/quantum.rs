use chrono::{NaiveTime, TimeDelta};

use crate::quantity::energy::WattHours;

pub trait Quantum<V> {
    /// Project the value into a bucket index.
    #[must_use]
    fn index(self, value: V) -> Option<usize>;
}

pub trait Midpoint<V> {
    /// Un-project the bucket index into the value that represents the middle of the bucket.
    #[must_use]
    fn midpoint(self, index: usize) -> V;
}

impl Quantum<NaiveTime> for TimeDelta {
    fn index(self, value: NaiveTime) -> Option<usize> {
        let nanos_since_midnight = (value - NaiveTime::MIN).num_nanoseconds()?;
        let quantum_nanos = self.num_nanoseconds()?;
        usize::try_from(nanos_since_midnight.checked_div(quantum_nanos)?).ok()
    }
}

impl Quantum<Self> for WattHours {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn index(self, value: Self) -> Option<usize> {
        let index = (value / self).floor();
        (index >= 0.0).then_some(index as usize)
    }
}

impl Midpoint<Self> for WattHours {
    #[expect(clippy::cast_precision_loss)]
    fn midpoint(self, index: usize) -> Self {
        self * (index as f64 + 0.5)
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
    fn quantize_energy() {
        assert_eq!(WattHours(1.0).index(WattHours(3.0)).unwrap(), 3);
        assert_eq!(WattHours(1.0).index(WattHours(3.0_f64.next_down())).unwrap(), 2);
    }

    #[test]
    fn dequantize_energy() {
        assert_eq!(WattHours(1.0).midpoint(2), WattHours(2.5));
    }
}
