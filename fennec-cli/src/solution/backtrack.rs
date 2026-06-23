use crate::{
    Schedule,
    energy,
    quantity::price::KilowattHourPrice,
    solution::{Metrics, Step},
};

pub struct Backtrack {
    pub metrics: Metrics,
    pub schedule: Schedule<(energy::Flow<KilowattHourPrice>, Step)>,
}
