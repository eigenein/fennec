use chrono::{DateTime, Local};

#[derive(Copy, Clone)]
pub struct Point<V> {
    pub time: DateTime<Local>,
    pub value: V,
}
