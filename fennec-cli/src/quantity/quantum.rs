use chrono::{NaiveTime, TimeDelta};

pub trait Quantum<V> {
    /// Project the value into a bucket index.
    #[must_use]
    fn index(self, value: V) -> usize;
}

impl Quantum<NaiveTime> for TimeDelta {
    fn index(self, value: NaiveTime) -> usize {
        let millis_since_midnight = (value - NaiveTime::MIN).num_milliseconds();
        let quantum_millis = self.num_milliseconds();
        let index = millis_since_midnight / quantum_millis;
        index.try_into().unwrap()
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
}
