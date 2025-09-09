use std::{collections::BTreeSet, iter::once};

use bon::Builder;
use itertools::Itertools;

pub use self::{
    optimizer::Optimizer,
    working_mode::{WorkingMode, WorkingModeHourlySchedule},
};
use crate::units::rate::KilowattHourRate;

mod optimizer;
mod simulator;
mod working_mode;

#[derive(Builder)]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct Strategy {
    /// Maximum rate when the battery is allowed to charge.
    ///
    /// [`None`] means never to charge.
    pub max_charging_rate: Option<KilowattHourRate>,

    /// Maximum rate when the battery is allowed to discharge.
    ///
    /// [`None`] means never to discharge.
    pub min_discharging_rate: Option<KilowattHourRate>,
}

impl<S: strategy_builder::IsComplete> StrategyBuilder<S> {
    pub fn build(self) -> Strategy {
        let this = self.build_internal();
        assert!(
            this.max_charging_rate
                .zip(this.min_discharging_rate)
                .is_none_or(|(max_charging_rate, min_discharging_rate)| max_charging_rate
                    < min_discharging_rate)
        );
        this
    }
}

impl Strategy {
    /// Generate all possible strategies given the unique future rates.
    pub fn iter_from_rates(rates: &BTreeSet<KilowattHourRate>) -> impl Iterator<Item = Self> {
        // Insert `None` at the beginning and the end to consider not charging and/or discharging.
        once(None)
            .chain(rates.iter().copied().map(Some))
            .chain(once(None))
            .array_combinations()
            .map(|[max_charging_rate, min_discharging_rate]| {
                Self::builder()
                    .maybe_max_charging_rate(max_charging_rate)
                    .maybe_min_discharging_rate(min_discharging_rate)
                    .build()
            })
    }
}
