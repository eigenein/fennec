use std::fmt::{Display, Formatter};

use comfy_table::{Attribute, Cell, Color, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{Quantity, cost::Cost, energy::KilowattHours},
};

#[must_use]
pub struct Solution {
    /// Cumulative loss till the end of the forecast period – our primary optimization target.
    pub cumulative_loss: Cost,

    /// Cumulative charge till the end of the forecast period.
    pub cumulative_charge: KilowattHours,

    /// Cumulative discharge till the end of the forecast period.
    pub cumulative_discharge: KilowattHours,

    /// TODO: consider incorporating [`Step`] into [`Solution`].
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self {
        cumulative_loss: Cost::ZERO,
        cumulative_charge: Quantity::ZERO,
        cumulative_discharge: Quantity::ZERO,
        step: None,
    };

    pub fn cumulative_energy_flow(&self) -> KilowattHours {
        self.cumulative_charge + self.cumulative_discharge
    }

    pub const fn with_base_loss(&self, base_loss: Cost) -> SolutionSummary<'_> {
        SolutionSummary { base_loss, solution: self }
    }
}

#[must_use]
pub struct SolutionSummary<'a> {
    solution: &'a Solution,

    /// Estimated loss without using the battery.
    base_loss: Cost,
}

impl SolutionSummary<'_> {
    fn profit(&self) -> Cost {
        self.base_loss - self.solution.cumulative_loss
    }
}

impl Display for SolutionSummary<'_> {
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
                Cell::from(self.profit() / self.solution.cumulative_energy_flow())
                    .add_attribute(Attribute::Bold),
                Cell::from(self.solution.cumulative_charge).fg(Color::Green),
                Cell::from(self.solution.cumulative_discharge).fg(Color::Red),
                Cell::from(self.base_loss),
                Cell::from(self.solution.cumulative_loss),
            ]);
        write!(f, "{table}")
    }
}
