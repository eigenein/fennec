use crate::prelude::*;

pub struct Forecast<M> {
    pub start_hour: usize,
    pub metrics: Vec<M>,
}

impl<M> Forecast<M> {
    pub fn try_extend(&mut self, other: Self) -> Result {
        let expected_hour = (self.start_hour + self.metrics.len()) % 24;
        ensure!(
            other.start_hour == expected_hour,
            "the other plan starts at the wrong hour: {actual} (expected {expected})",
            expected = expected_hour,
            actual = other.start_hour,
        );
        self.metrics.extend(other.metrics);
        Ok(())
    }

    pub fn try_zip<R, T>(self, rhs: Forecast<R>, f: fn(M, R) -> T) -> Result<Forecast<T>> {
        ensure!(
            self.start_hour == rhs.start_hour,
            "both plans start at different hours: {} vs {}",
            self.start_hour,
            rhs.start_hour,
        );
        let metrics =
            self.metrics.into_iter().zip(rhs.metrics).map(|(lhs, rhs)| f(lhs, rhs)).collect();
        Ok(Forecast { start_hour: self.start_hour, metrics })
    }
}

impl<M> Forecast<M>
where
    M: Copy,
{
    pub fn iter(&self) -> impl Iterator<Item = (usize, M)> {
        (self.start_hour..).zip(&self.metrics).map(|(hour, metrics)| (hour % 24, *metrics))
    }
}
