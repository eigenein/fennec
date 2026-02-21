use comfy_table::{Cell, Color, Table, modifiers, presets};
use derive_more::Add;

use crate::{
    core::{flow::Flow, solution::Losses, working_mode::WorkingMode},
    quantity::{Zero, currency::Mills, energy::WattHours},
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

impl Metrics {
    pub fn into_table(self, base_loss: Mills) -> Table {
        let profit = base_loss - self.losses.total();
        let mut table = Table::new();
        let profit_color = if profit > Mills::ZERO { Color::Green } else { Color::Red };
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Profit").fg(profit_color),
                Cell::from("Base\nloss"),
                Cell::from("Grid\nloss"),
                Cell::from("Battery\ncharge").fg(WorkingMode::Charge.color()),
                Cell::from("Battery\ndischarge").fg(WorkingMode::Discharge.color()),
                Cell::from("Battery\nloss"),
            ])
            .add_row(vec![
                Cell::from(profit).fg(profit_color),
                Cell::from(base_loss),
                Cell::from(self.losses.grid),
                Cell::from(self.internal_battery_flow.import).fg(WorkingMode::Charge.color()),
                Cell::from(self.internal_battery_flow.export).fg(WorkingMode::Discharge.color()),
                Cell::from(self.losses.battery),
            ]);
        table
    }
}
