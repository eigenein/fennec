use crate::{
    battery,
    energy::Flow,
    ops::Interval,
    quantity::{currency::Mills, power::Watts, price::KilowattHourPrice},
    solution::{Metrics, Step},
};

#[must_use]
pub struct HunterState {
    pub steps: Vec<((Interval, Flow<KilowattHourPrice>), Step)>,
    pub base_loss: Mills,
    pub metrics: Metrics,
    pub average_eps_power: Watts,
    pub battery_efficiency: battery::Efficiency,
}

impl HunterState {
    pub fn profit(&self) -> Mills {
        self.base_loss - self.metrics.losses.total()
    }
}

#[must_use]
pub struct LoggerState {
    pub battery: battery::State,
}
