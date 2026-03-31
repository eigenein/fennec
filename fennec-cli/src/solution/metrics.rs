use derive_more::Add;

use crate::{
    energy::Flow,
    quantity::{Zero, energy::WattHours},
    solution::Losses,
};

#[must_use]
#[derive(Copy, Clone, Add)]
pub struct Metrics {
    pub internal_battery_flow: Flow<WattHours>,
    pub losses: Losses,
}

impl Zero for Metrics {
    const ZERO: Self = Self { internal_battery_flow: Flow::ZERO, losses: Losses::ZERO };
}
