use std::fmt::{Display, Formatter};

use comfy_table::{Cell, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{Zero, currency::Mills},
};

#[must_use]
pub struct Solution {
    /// Cumulative loss till the end of the forecast period – our primary optimization target.
    pub loss: Mills,

    /// First step associated with this solution.
    ///
    /// Boundary solutions have [`None`] here.
    pub step: Option<Step>,
}

impl Solution {
    /// Empty solution that is returned for the time interval beyond the forecast horizon.
    pub const BOUNDARY: Self = Self { loss: Mills::ZERO, step: None };
}

#[must_use]
pub struct SolutionSummary {
    pub loss: Mills,

    /// Estimated loss without using the battery.
    pub base_loss: Mills,
}

impl SolutionSummary {
    fn profit(&self) -> Mills {
        self.base_loss - self.loss
    }
}

impl Display for SolutionSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![Cell::from("Net profit"), Cell::from("Base loss"), Cell::from("Loss")])
            .add_row(vec![
                Cell::from(self.profit()),
                Cell::from(self.base_loss),
                Cell::from(self.loss),
            ]);
        write!(f, "{table}")
    }
}
