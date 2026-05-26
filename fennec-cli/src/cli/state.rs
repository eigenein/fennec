use crate::{
    Interval,
    battery,
    energy,
    energy::Flow,
    quantity::{power::Watts, price::KilowattHourPrice},
    solution::{Metrics, Step},
};

#[must_use]
pub struct Hunter {
    pub steps: Vec<((Interval, Flow<KilowattHourPrice>), Step)>,
    pub metrics: Metrics,
    pub average_eps_power: Watts,
    pub battery_efficiency: battery::Efficiency,
}

#[must_use]
pub struct Logger {
    pub battery: battery::State,
    pub energy_profile: energy::NewProfile,
}
