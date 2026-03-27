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
pub struct EnergyState {
    pub design_capacity: DecawattHours,
    pub state_of_charge: Percentage,
    pub state_of_health: Percentage,
    pub active_power: Watts,
}

impl EnergyState {
    /// Battery capacity corrected on the state of health.
    pub fn actual_capacity(&self) -> WattHours {
        WattHours::from(self.design_capacity) * self.state_of_health
    }

    /// Residual energy corrected on the state of health.
    pub fn residual(&self) -> WattHours {
        self.actual_capacity() * self.state_of_charge
    }
}

/// TODO: union everything into this state. Split when the separation would become clear.
#[must_use]
pub struct FullState {
    pub energy: EnergyState,
    pub allowed_state_of_charge: range::Inclusive<Percentage>,
}

impl FullState {
    pub fn min_residual_energy(&self) -> WattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.min
    }

    pub fn max_residual_energy(&self) -> WattHours {
        self.energy.actual_capacity() * self.allowed_state_of_charge.max
    }
}

impl Display for FullState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Table::new()
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![Cell::from("Battery")])
            .add_row(vec![
                Cell::from("Residual energy").fg(Color::Green),
                Cell::from(self.energy.residual())
                    .fg(Color::Green)
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("Design capacity"),
                Cell::from(self.energy.design_capacity).set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("State of charge").fg(Color::Green),
                Cell::from(self.energy.state_of_charge)
                    .fg(Color::Green)
                    .set_alignment(CellAlignment::Right),
            ])
            .add_row(vec![
                Cell::from("State of health"),
                Cell::from(self.energy.state_of_health).set_alignment(CellAlignment::Right),
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
