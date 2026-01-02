mod aggregate;
mod differentiate;

pub use self::{aggregate::Aggregate, differentiate::Differentiate};

#[deprecated]
pub type Point<K, V> = (K, V);
