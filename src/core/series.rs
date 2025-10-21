mod average;
mod differentiate;
mod resample;

pub use self::{average::AverageHourly, differentiate::Differentiate, resample::Resample};

pub type Point<K, V> = (K, V);
pub type Series<K, V> = Vec<Point<K, V>>;
