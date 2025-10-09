use std::iter::Sum;

impl<T> SumValues for T where T: ?Sized {}

pub trait SumValues {
    fn sum_values<K, V>(self) -> V
    where
        Self: Iterator<Item = (K, V)> + Sized,
        V: Sum,
    {
        self.map(|(_, value)| value).sum::<V>()
    }
}
