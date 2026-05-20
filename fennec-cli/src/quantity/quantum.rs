use chrono::{NaiveTime, TimeDelta};

use crate::quantity::energy::WattHours;

pub trait Quantum<V> {
    /// Project the value into a bucket index.
    #[must_use]
    fn index(self, value: V) -> usize;
}

pub trait Midpoint<V> {
    /// Un-project the bucket index into the value that represents the middle of the bucket.
    #[must_use]
    fn midpoint(self, index: usize) -> V;
}

impl Quantum<NaiveTime> for TimeDelta {
    fn index(self, value: NaiveTime) -> usize {
        let millis_since_midnight = (value - NaiveTime::MIN).num_milliseconds();
        let quantum_millis = self.num_milliseconds();
        let index = millis_since_midnight / quantum_millis;
        index.try_into().unwrap()
    }
}

impl Quantum<Self> for WattHours {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn index(self, value: Self) -> usize {
        let index = (value / self).floor();
        assert!(index >= 0.0);
        index as usize
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
    fn naive_time_index() {
        let time = NaiveTime::from_hms_opt(12, 10, 0).unwrap();
        assert_eq!(TimeDelta::minutes(5).index(time), 12 * 12 + 2);
    }

    #[test]
    fn max_naive_time_index() {
        let secs = 23 * 3600 + 59 * 60 + 59;
        let naive_time = NaiveTime::from_num_seconds_from_midnight_opt(secs, 999_999_999).unwrap();
        assert_eq!(TimeDelta::minutes(15).index(naive_time), 95);
    }

    #[test]
    fn energy_index() {
        assert_eq!(WattHours(1.0).index(WattHours(3.0)), 3);
        assert_eq!(WattHours(1.0).index(WattHours(3.0_f64.next_down())), 2);
    }

    #[test]
    fn energy_midpoint() {
        assert_eq!(WattHours(1.0).midpoint(2), WattHours(2.5));
    }
}
