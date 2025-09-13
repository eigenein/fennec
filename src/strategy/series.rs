use crate::{prelude::*, strategy::Point};

#[derive(derive_more::AsRef, derive_more::AsMut, derive_more::From, derive_more::IntoIterator)]
pub struct Series<M>(Vec<Point<M>>);

impl<M> Series<M> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Point<M>> {
        self.0.iter()
    }

    pub fn try_zip_by_time<R>(
        self,
        rhs: impl IntoIterator<Item = Point<R>>,
    ) -> Result<Series<(M, R)>> {
        self.0
            .into_iter()
            .zip(rhs)
            .map(|(lhs, rhs)| {
                if lhs.time == rhs.time {
                    Ok(Point { time: lhs.time, metrics: (lhs.metrics, rhs.metrics) })
                } else {
                    bail!("the timestamps differ: {:?} vs {:?}", lhs.time, rhs.time)
                }
            })
            .collect::<Result<_>>()
            .map(Series)
    }
}
