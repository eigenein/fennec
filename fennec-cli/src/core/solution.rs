use std::fmt::{Display, Formatter};

use comfy_table::{Attribute, Cell, Color, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{Quantity, cost::Cost, energy::KilowattHours},
};

#[must_use]
pub struct Solution {
    pub cumulative_metrics: CumulativeMetrics,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self { cumulative_metrics: CumulativeMetrics::ZERO, step: None };
}

#[must_use]
pub struct CumulativeMetrics {
    /// Cumulative loss till the end of the forecast period – our primary optimization target.
    pub loss: Cost,

    /// Cumulative charge till the end of the forecast period.
    pub charge: KilowattHours,

    /// Cumulative discharge till the end of the forecast period.
    pub discharge: KilowattHours,
}

impl CumulativeMetrics {
    pub const ZERO: Self =
        Self { loss: Quantity::ZERO, charge: Quantity::ZERO, discharge: Quantity::ZERO };

    pub fn energy_flow(&self) -> KilowattHours {
        self.charge + self.discharge
    }

    pub const fn with_base_loss(self, base_loss: Cost) -> SolutionSummary {
        SolutionSummary { base_loss, cumulative_metrics: self }
    }
}

#[must_use]
pub struct SolutionSummary {
    cumulative_metrics: CumulativeMetrics,

    /// Estimated loss without using the battery.
    base_loss: Cost,
}

impl SolutionSummary {
    fn profit(&self) -> Cost {
        self.base_loss - self.cumulative_metrics.loss
    }
}

impl Display for SolutionSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Net profit"),
                Cell::from("Flow profit"),
                Cell::from("Charge"),
                Cell::from("Discharge"),
                Cell::from("Base loss"),
                Cell::from("Loss"),
            ])
            .add_row(vec![
                Cell::from(self.profit()),
                Cell::from(self.profit() / self.cumulative_metrics.energy_flow())
                    .add_attribute(Attribute::Bold),
                Cell::from(self.cumulative_metrics.charge).fg(Color::Green),
                Cell::from(self.cumulative_metrics.discharge).fg(Color::Red),
                Cell::from(self.base_loss),
                Cell::from(self.cumulative_metrics.loss),
            ]);
        write!(f, "{table}")
    }
}
