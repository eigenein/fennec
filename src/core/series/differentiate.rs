use std::ops::{Div, Range, Sub};

use itertools::Itertools;

impl<T> Differentiate for T where T: ?Sized {}

pub trait Differentiate {
    /// Differentiate the values by the keys and return the iterator over `K` and `(ΔK, ΔV)`.
    fn deltas<K, V>(
        self,
    ) -> impl Iterator<Item = (Range<K>, (<K as Sub>::Output, <V as Sub>::Output))>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub + PartialEq,
        V: Copy + Sub,
    {
        self.tuple_windows().filter_map(|((from_index, from_value), (to_index, to_value))| {
            (from_index != to_index)
                .then_some(((from_index..to_index), (to_index - from_index, to_value - from_value)))
        })
    }

    fn differentiate<K, V>(
        self,
    ) -> impl Iterator<Item = (Range<K>, <<V as Sub>::Output as Div<<K as Sub>::Output>>::Output)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub + PartialEq,
        V: Copy + Sub,
        <V as Sub>::Output: Div<<K as Sub>::Output>,
    {
        self.deltas().map(|(index_range, (index_delta, value_delta))| {
            (index_range, value_delta / index_delta)
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
        assert_eq!(diff, vec![(2..3, (1, 100)), (3..5, (2, 400))]);
    }
}
