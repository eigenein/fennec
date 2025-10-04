use std::ops::{Div, Sub};

use itertools::Itertools;

impl<K, V, T> Differentiate<K, V> for T where T: ?Sized {}

pub trait Differentiate<K, V> {
    fn differentiate<R>(self) -> impl Iterator<Item = (K, R)>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Clone + Sub<K>,
        V: Clone + Sub<V>,
        <V as Sub<V>>::Output: Div<<K as Sub<K>>::Output, Output = R>,
    {
        self.tuple_windows().map(|((from_index, from_value), (to_index, to_value))| {
            (from_index.clone(), (to_value - from_value) / (to_index - from_index))
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_differentiate() {
        let series = vec![(1, 100), (2, 200), (4, 600)];
        let diff: Vec<_> = series.into_iter().differentiate().collect();
        assert_eq!(diff, vec![(1, 100), (2, 200)]);
    }
}
