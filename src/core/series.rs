mod average;
mod differentiate;
mod resample;
mod sum;

pub use self::{
    average::AverageHourly,
    differentiate::Differentiate,
    resample::Resample,
    sum::SumValues,
};

pub type Point<K, V> = (K, V);
pub type Series<K, V> = Vec<Point<K, V>>;
