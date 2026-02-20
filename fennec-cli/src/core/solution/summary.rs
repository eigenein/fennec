use std::fmt::{Display, Formatter};

use comfy_table::{Cell, Table, modifiers, presets};

use crate::quantity::currency::Mills;

#[must_use]
pub struct Summary {
    pub grid_loss: Mills,

    /// Estimated loss without using the battery.
    pub base_loss: Mills,
}

impl Summary {
    fn profit(&self) -> Mills {
        self.base_loss - self.grid_loss
    }
}

impl Display for Summary {
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
                Cell::from(self.grid_loss),
            ]);
        write!(f, "{table}")
    }
}
