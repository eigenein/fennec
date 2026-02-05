use std::fmt::{Display, Formatter};

use comfy_table::{Attribute, Cell, Color, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{Quantity, cost::Cost, energy::KilowattHours},
};

#[must_use]
pub struct Solution {
    /// Net loss till the end of the forecast period – our primary optimization target.
    pub net_loss: Cost,

    /// Cumulative charge till the end of the forecast period.
    pub charge: KilowattHours,

    /// Cumulative discharge till the end of the forecast period.
    pub discharge: KilowattHours,

    /// TODO: consider incorporating [`Step`] into [`Solution`].
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self {
        net_loss: Cost::ZERO,
        charge: Quantity::ZERO,
        discharge: Quantity::ZERO,
        step: None,
    };

    pub fn energy_flow(&self) -> KilowattHours {
        self.charge + self.discharge
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
        self.base_loss - self.solution.net_loss
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
                Cell::from(self.profit() / self.solution.energy_flow())
                    .add_attribute(Attribute::Bold),
                Cell::from(self.solution.charge).fg(Color::Green),
                Cell::from(self.solution.discharge).fg(Color::Red),
                Cell::from(self.base_loss),
                Cell::from(self.solution.net_loss),
            ]);
        write!(f, "{table}")
    }
}
