use chrono::{DateTime, Local};

#[derive(Copy, Clone)]
pub struct Point<M> {
    pub time: DateTime<Local>,
    pub metrics: M,
}
