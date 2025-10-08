mod average;
mod battery;
mod differentiate;

pub use self::{
    average::AverageHourly,
    battery::{BatteryParameters, TryEstimateBatteryParameters},
    differentiate::Differentiate,
};

pub type Point<K, V> = (K, V);
pub type Series<K, V> = Vec<Point<K, V>>;
