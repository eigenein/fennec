use crate::{
    battery,
    energy,
    energy::Flow,
    ops::Interval,
    quantity::{power::Watts, price::KilowattHourPrice},
    solution::{Metrics, Step},
};

#[must_use]
pub struct HunterState {
    pub steps: Vec<((Interval, Flow<KilowattHourPrice>), Step)>,
    pub metrics: Metrics,
    pub average_eps_power: Watts,
    pub battery_efficiency: battery::Efficiency,
}

#[must_use]
pub struct LoggerState {
    pub battery: battery::State,
    pub energy_profile: energy::NewProfile,
}
