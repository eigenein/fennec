use std::ops::{Div, Range, Sub};

use itertools::Itertools;

impl<T> Differentiate for T where T: ?Sized {}

pub trait Differentiate {
    /// Differentiate the values by the keys and return the iterator over `(Range<K>, ΔV)`.
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

    fn differentiate<K, DV>(
        self,
    ) -> impl Iterator<Item = (Range<K>, <DV as Div<<K as Sub>::Output>>::Output)>
    where
        Self: Sized + IntoIterator<Item = (Range<K>, DV)>,
        K: Clone + Sub,
        DV: Div<<K as Sub>::Output>,
    {
        self.into_iter().map(|(index_range, value_delta)| {
            (index_range.clone(), value_delta / (index_range.end - index_range.start))
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
