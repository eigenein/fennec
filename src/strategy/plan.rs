use crate::{
    prelude::*,
    strategy::{Metrics, Point, Step},
};

pub struct Plan {
    pub metrics: Metrics,
    pub step: Step,
}

impl Point<Plan> {
    pub fn try_from(zip: (Point<Metrics>, Point<Step>)) -> Result<Self> {
        let (metrics_point, step_point) = zip;
        ensure!(metrics_point.time == step_point.time);
        Ok(Self {
            time: metrics_point.time,
            value: Plan { metrics: metrics_point.value, step: step_point.value },
        })
    }
}
