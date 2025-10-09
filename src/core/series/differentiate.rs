use std::ops::{Div, Sub};

use itertools::Itertools;

impl<T> Differentiate for T where T: ?Sized {}

pub trait Differentiate {
    /// Differentiate the values by the keys and return the iterator over `K` and `(ΔV, ΔK)`.
    #[expect(clippy::type_complexity)]
    fn deltas<K, V>(
        self,
    ) -> impl Iterator<Item = (K, (<V as Sub<V>>::Output, <K as Sub<K>>::Output))>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Clone + Sub<K>,
        V: Clone + Sub<V>,
    {
        self.tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (from_index.clone(), (to_value - from_value, to_index - from_index))
        })
    }

    /// Differentiate the values by the keys and return the iterator over `K` and `ΔV/ΔK`.
    fn differentiate<K, V, R>(self) -> impl Iterator<Item = (K, R)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Clone + Sub<K>,
        <K as Sub<K>>::Output: Clone,
        V: Clone + Sub<V>,
        <V as Sub<V>>::Output: Div<<K as Sub<K>>::Output, Output = R>,
    {
        self.deltas().map(|(from_index, (delta_v, delta_k))| (from_index, delta_v / delta_k))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_differentiate() {
        let series = vec![(2, 100), (3, 200), (5, 600)];
        let diff: Vec<_> = series.into_iter().differentiate().collect();
        assert_eq!(diff, vec![(2, 100), (3, 200)]);
    }
}
