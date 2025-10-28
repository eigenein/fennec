mod aggregate;
mod differentiate;

pub use self::{aggregate::AggregateHourly, differentiate::Differentiate};

pub type Point<K, V> = (K, V);
pub type Series<K, V> = Vec<Point<K, V>>;
