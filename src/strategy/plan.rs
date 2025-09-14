use crate::strategy::{Metrics, Point, Step};

pub struct Plan {
    pub metrics: Metrics,
    pub step: Step,
}

impl From<(Point<Metrics>, Point<Step>)> for Point<Plan> {
    fn from(pair: (Point<Metrics>, Point<Step>)) -> Self {
        let (metrics_point, step_point) = pair;
        assert_eq!(metrics_point.time, step_point.time);
        Self {
            time: metrics_point.time,
            value: Plan { metrics: metrics_point.value, step: step_point.value },
        }
    }
}
