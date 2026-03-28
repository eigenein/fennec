use std::fmt::{Display, Formatter};

use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    ops::range,
    quantity::{
        energy::{DecawattHours, WattHours},
        power::Watts,
        ratios::Percentage,
    },
};

#[must_use]
pub struct State {
    /// State-of-charge (SoC) percentage.
    pub charge: Percentage,

    /// State-of-health (SoH) percentage.
    pub health: Percentage,

    /// Design capacity – constant for the product lifetime.
    pub design_capacity: DecawattHours,

    /// Allowed on-grid SoC levels.
    pub allowed_state_of_charge: range::Inclusive<Percentage>,

    /// Battery active power.
    ///
    /// Positive means discharging, negative means charging.
    pub battery_active_power: Watts,

    /// Active power on the EPS output.
    pub eps_active_power: Watts,
}

impl State {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        WattHours::from(self.design_capacity) * self.health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.charge
    }

    pub fn min_residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.allowed_state_of_charge.min
    }

    pub fn max_residual_energy(&self) -> WattHours {
        self.actual_capacity() * self.allowed_state_of_charge.max
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Table::new()
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![Cell::from("Battery")])
            .add_row(vec![
                Cell::from("Residual energy").fg(Color::Green),
                Cell::from(self.residual_energy())
                    .fg(Color::Green)
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Design capacity"),
                Cell::from(self.design_capacity).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("State of charge").fg(Color::Green),
                Cell::from(self.charge).fg(Color::Green).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("State of health"),
                Cell::from(self.health).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Minimum SoC"),
                Cell::from(self.allowed_state_of_charge.min).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Maximum SoC"),
                Cell::from(self.allowed_state_of_charge.max).set_alignment(CellAlignment::Right),
            ])
            .fmt(f)
    }
}
