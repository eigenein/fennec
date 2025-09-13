use crate::prelude::*;

pub struct HourlySeries<M> {
    pub start_hour: usize,
    pub points: Vec<M>,
}

impl<M> HourlySeries<M> {
    pub fn try_extend(&mut self, other: Self) -> Result {
        let expected_hour = (self.start_hour + self.points.len()) % 24;
        ensure!(
            other.start_hour == expected_hour,
            "the other plan starts at the wrong hour: {actual} (expected {expected})",
            expected = expected_hour,
            actual = other.start_hour,
        );
        self.points.extend(other.points);
        Ok(())
    }

    pub fn try_zip<R: Copy, T>(
        self,
        rhs: &HourlySeries<R>,
        f: fn(M, R) -> T,
    ) -> Result<HourlySeries<T>> {
        ensure!(
            self.start_hour == rhs.start_hour,
            "both plans start at different hours: {} vs {}",
            self.start_hour,
            rhs.start_hour,
        );
        let metrics =
            self.points.into_iter().zip(&rhs.points).map(|(lhs, rhs)| f(lhs, *rhs)).collect();
        Ok(HourlySeries { start_hour: self.start_hour, points: metrics })
    }
}

impl<M> HourlySeries<M>
where
    M: Copy,
{
    pub fn iter(&self) -> impl Iterator<Item = (usize, M)> {
        (self.start_hour..).zip(&self.points).map(|(hour, metrics)| (hour % 24, *metrics))
    }
}
