use std::fmt::{Display, Formatter};

use comfy_table::{Cell, Table, modifiers, presets};

use crate::{core::solution::Losses, quantity::currency::Mills};

#[must_use]
pub struct Summary {
    pub losses: Losses,

    /// Estimated loss without using the battery.
    pub base_loss: Mills,
}

impl Summary {
    fn profit(&self) -> Mills {
        self.base_loss - self.losses.total()
    }
}

impl Display for Summary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Net profit"),
                Cell::from("Base loss"),
                Cell::from("Grid loss"),
                Cell::from("Battery loss"),
            ])
            .add_row(vec![
                Cell::from(self.profit()),
                Cell::from(self.base_loss),
                Cell::from(self.losses.grid),
                Cell::from(self.losses.battery),
            ]);
        write!(f, "{table}")
    }
}
