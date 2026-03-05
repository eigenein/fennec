use average::Mean;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    core::{step::Step, working_mode::WorkingMode},
    quantity::{Zero, currency::Mills, energy::WattHours, price::KilowattHourPrice},
};

pub fn build_steps_table(steps: &[Step]) -> Table {
    let average_price: KilowattHourPrice = {
        let estimate: Mean = steps.iter().map(|step| step.energy_price.0).collect();
        if estimate.is_empty() {
            KilowattHourPrice::ZERO
        } else {
            KilowattHourPrice(estimate.mean())
        }
    };

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL_CONDENSED)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .enforce_styling()
        .set_header(vec![
            Cell::new("Date"),
            Cell::new("Start\ntime"),
            Cell::new("End\ntime"),
            Cell::new("Energy\nprice"),
            Cell::new("Mode"),
            Cell::new("Grid\nimport"),
            Cell::new("Grid\nexport"),
            Cell::new("Battery\nimport").fg(WorkingMode::Charge.color()),
            Cell::new("Battery\nexport").fg(WorkingMode::Discharge.color()),
            Cell::new("Residual\nafter"),
            Cell::new("Grid\nloss"),
            Cell::new("Battery\nloss"),
        ]);
    for step in steps {
        table.add_row(vec![
            Cell::new(step.interval.start.format("%b %d")).add_attribute(Attribute::Dim),
            Cell::new(step.interval.start.format("%H:%M")),
            Cell::new(step.interval.end.format("%H:%M")).add_attribute(Attribute::Dim),
            Cell::new(step.energy_price).fg(if step.energy_price >= average_price {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(step.working_mode).fg(step.working_mode.color()),
            Cell::new(step.energy_balance.grid.import).set_alignment(CellAlignment::Right).fg(
                if step.energy_balance.grid.import > WattHours::ONE {
                    Color::Red
                } else {
                    Color::Green
                },
            ),
            Cell::new(step.energy_balance.grid.export).set_alignment(CellAlignment::Right).fg(
                if step.energy_balance.grid.export > WattHours::ONE {
                    Color::Blue
                } else {
                    Color::Reset
                },
            ),
            Cell::new(step.energy_balance.battery.import)
                .fg(if step.energy_balance.battery.import > WattHours::ONE {
                    WorkingMode::Charge.color()
                } else {
                    Color::Reset
                })
                .set_alignment(CellAlignment::Right),
            Cell::new(step.energy_balance.battery.export)
                .fg(if step.energy_balance.battery.export > WattHours::ONE {
                    WorkingMode::Discharge.color()
                } else {
                    Color::Reset
                })
                .set_alignment(CellAlignment::Right),
            Cell::new(step.residual_energy_after).set_alignment(CellAlignment::Right),
            Cell::new(step.metrics.losses.grid)
                .set_alignment(CellAlignment::Right)
                .fg(if step.metrics.losses.grid >= Mills::TEN { Color::Red } else { Color::Green }),
            Cell::new(step.metrics.losses.battery).set_alignment(CellAlignment::Right).fg(
                if step.metrics.losses.battery >= Mills::TEN { Color::Red } else { Color::Green },
            ),
        ]);
    }
    table
}
