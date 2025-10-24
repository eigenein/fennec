use std::ops::{Div, Sub};

use itertools::Itertools;

impl<T> Differentiate for T where T: ?Sized {}

pub trait Differentiate {
    /// Differentiate the values by the keys and return the iterator over `K` and `(ΔK, ΔV)`.
    fn deltas<K, V>(self) -> impl Iterator<Item = (K, (<K as Sub>::Output, <V as Sub>::Output))>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub,
        V: Copy + Sub,
    {
        self.tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (from_index, (to_index - from_index, to_value - from_value))
        })
    }

    fn differentiate<K, V>(
        self,
    ) -> impl Iterator<Item = (K, <<V as Sub>::Output as Div<<K as Sub>::Output>>::Output)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Copy + Sub,
        V: Copy + Sub,
        <V as Sub>::Output: Div<<K as Sub>::Output>,
    {
        self.deltas()
            .map(|(timestamp, (index_delta, value_delta))| (timestamp, value_delta / index_delta))
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
