use std::collections::BTreeSet;

use bon::Builder;
use itertools::Itertools;

pub use self::{
    optimizer::Optimizer,
    schedule::WorkingModeHourlySchedule,
    working_mode::WorkingMode,
};
use crate::units::KilowattHourRate;

mod optimizer;
mod schedule;
mod working_mode;

#[derive(Copy, Clone, Builder)]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct Strategy {
    /// Maximum rate when the battery is allowed to charge.
    pub max_charging_rate: KilowattHourRate,

    /// Maximum rate when the battery is allowed to discharge.
    pub min_discharging_rate: KilowattHourRate,
}

impl<S: strategy_builder::IsComplete> StrategyBuilder<S> {
    pub fn build(self) -> Strategy {
        let this = self.build_internal();
        assert!(this.max_charging_rate < this.min_discharging_rate);
        this
    }
}

impl Strategy {
    /// Generate all possible strategies given the unique future rates.
    pub fn iter_from_rates(rates: &BTreeSet<KilowattHourRate>) -> impl Iterator<Item = Self> {
        rates.iter().copied().array_combinations().map(
            |[max_charging_rate, min_discharging_rate]| {
                Self::builder()
                    .max_charging_rate(max_charging_rate)
                    .min_discharging_rate(min_discharging_rate)
                    .build()
            },
        )
    }
}
