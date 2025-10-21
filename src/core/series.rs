mod aggregate;
mod differentiate;
mod resample;

pub use self::{aggregate::AggregateHourly, differentiate::Differentiate, resample::Resample};

pub type Point<K, V> = (K, V);
pub type Series<K, V> = Vec<Point<K, V>>;
