use std::ops::Sub;

use itertools::Itertools;

impl<T> Differentiate for T where T: ?Sized {}

pub trait Differentiate {
    /// Differentiate the values by the keys and return the iterator over `K` and `(ΔK, ΔV)`.
    #[expect(clippy::type_complexity)]
    fn deltas<K, V>(
        self,
    ) -> impl Iterator<Item = (K, (<K as Sub<K>>::Output, <V as Sub<V>>::Output))>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub<K>,
        V: Copy + Sub<V>,
    {
        self.tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (from_index, (to_index - from_index, to_value - from_value))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_differentiate() {
        let series = vec![(2, 100), (3, 200), (5, 600)];
        let diff: Vec<_> = series.into_iter().deltas().collect();
        assert_eq!(diff, vec![(2, (1, 100)), (3, (2, 400))]);
    }
}
