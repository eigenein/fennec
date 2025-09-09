use std::collections::BTreeSet;

use bon::Builder;
use itertools::Itertools;

pub use self::{
    optimizer::Optimization,
    working_mode::{WorkingMode, WorkingModeHourlySchedule},
};
use crate::units::rate::KilowattHourRate;

mod optimizer;
mod simulator;
mod working_mode;

#[derive(Builder)]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct Strategy {
    /// Maximum rate when the battery is allowed to charge unconditionally.
    pub max_charging_rate: KilowattHourRate,

    /// Maximum rate when the battery is allowed to discharge unconditionally.
    pub min_discharging_rate: KilowattHourRate,
}

impl<S: strategy_builder::IsComplete> StrategyBuilder<S> {
    pub fn build(self) -> Strategy {
        let this = self.build_internal();
        assert!(this.max_charging_rate <= this.min_discharging_rate);
        this
    }
}

impl Strategy {
    /// Generate all possible strategies given the unique future rates.
    pub fn iter_from_rates(rates: BTreeSet<KilowattHourRate>) -> impl Iterator<Item = Self> {
        rates.into_iter().combinations_with_replacement(2).map(|combination| {
            Self::builder()
                .max_charging_rate(combination[0])
                .min_discharging_rate(combination[1])
                .build()
        })
    }
}
