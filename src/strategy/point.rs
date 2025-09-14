use chrono::{DateTime, Local};

#[derive(Copy, Clone)]
pub struct Point<V> {
    pub time: DateTime<Local>,
    pub value: V,
}

impl<V> Point<V> {
    pub fn mapper<T>(map: fn(V) -> T) -> impl Fn(Self) -> Point<T> {
        move |this| Point { time: this.time, value: map(this.value) }
    }
}
