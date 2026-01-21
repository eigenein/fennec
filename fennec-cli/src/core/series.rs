use std::ops::{Range, Sub};

use itertools::Itertools;

impl<T> Deltas for T where T: ?Sized {}

pub trait Deltas {
    /// Subtract the pairwise windows and return the iterator over `(Range<K>, ΔV)`.
    fn deltas<K, V>(self) -> impl Iterator<Item = (Range<K>, <V as Sub>::Output)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy,
        V: Copy + Sub,
    {
        self.tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (from_index..to_index, to_value - from_value)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deltas() {
        let series = vec![(2, 100), (3, 200), (5, 600)];
        let diff: Vec<_> = series.into_iter().deltas().collect();
        assert_eq!(diff, vec![(2..3, 100), (3..5, 400)]);
    }
}
