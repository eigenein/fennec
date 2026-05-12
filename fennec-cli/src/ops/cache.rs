use std::time::Instant;

#[must_use]
pub struct Entry<T> {
    pub timestamp: Instant,
    pub value: T,
}

impl<T> Entry<T> {
    pub fn now(value: T) -> Self {
        Self { timestamp: Instant::now(), value }
    }
}
