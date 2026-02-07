use average::Mean;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{cost::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

pub fn build_steps_table(steps: &[Step], battery_discharging_power: Kilowatts) -> Table {
    let mean_rate: KilowattHourRate = {
        let estimate: Mean = steps.iter().map(|step| step.grid_rate.0).collect();
        if estimate.is_empty() { KilowattHourRate::ZERO } else { estimate.mean().into() }
    };

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL_CONDENSED)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .enforce_styling();
    table.set_header(vec![
        "Date", "Start", "End", "Grid", "Net", "Mode", "Before", "After", "Grid", "Loss",
    ]);
    for step in steps {
        table.add_row(vec![
            Cell::new(step.interval.start.format("%b %d")).add_attribute(Attribute::Dim),
            Cell::new(step.interval.start.format("%H:%M")),
            Cell::new(step.interval.end.format("%H:%M")).add_attribute(Attribute::Dim),
            Cell::new(step.grid_rate).fg(if step.grid_rate >= mean_rate {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(step.stand_by_power).set_alignment(CellAlignment::Right).fg(
                if step.stand_by_power <= Kilowatts::ZERO {
                    Color::Green
                } else if step.stand_by_power <= battery_discharging_power {
                    Color::DarkYellow
                } else {
                    Color::Red
                },
            ),
            Cell::new(step.working_mode).fg(step.working_mode.color()),
            Cell::new(step.residual_energy_before)
                .set_alignment(CellAlignment::Right)
                .add_attribute(Attribute::Dim),
            Cell::new(step.residual_energy_after).set_alignment(CellAlignment::Right),
            Cell::new(step.grid_consumption).set_alignment(CellAlignment::Right).fg(
                if step.grid_consumption >= KilowattHours::ONE_WATT_HOUR {
                    Color::Red
                } else {
                    Color::Green
                },
            ),
            Cell::new(step.loss)
                .set_alignment(CellAlignment::Right)
                .fg(if step.loss >= Cost::ONE_CENT { Color::Red } else { Color::Green }),
        ]);
    }
    table
}
