use average::Mean;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};
use fennec_modbus::contrib::mq2200::schedule;

use crate::{
    battery::WorkingMode,
    quantity::{Zero, currency::Mills, energy::WattHours, price::KilowattHourPrice},
    solution::Step,
};

pub fn build_steps_table(steps: &[Step]) -> Table {
    let average_price: KilowattHourPrice = {
        let estimate: Mean = steps.iter().map(|step| step.energy_price.import.0).collect();
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
            Cell::new("Duration"),
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
            Cell::new(step.duration).add_attribute(Attribute::Dim),
            Cell::new(step.energy_price.import).fg(if step.energy_price.import >= average_price {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(step.working_mode)
                .fg(step.working_mode.color())
                .add_attribute(step.working_mode.attribute()),
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

pub fn build_fox_ess_schedule_table(entries: &schedule::Full) -> Table {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL_CONDENSED)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .enforce_styling()
        .set_header(vec![
            Cell::new("Start\ntime"),
            Cell::new("End\ntime"),
            Cell::new("Enabled"),
            Cell::new("Mode"),
            Cell::new("Target\nSoC"),
            Cell::new("Watts"),
        ]);
    for entry in entries {
        let attribute = if entry.is_enabled { Attribute::Reset } else { Attribute::Dim };
        table.add_row(vec![
            Cell::new(entry.start_time).add_attribute(attribute),
            Cell::new(entry.end_time).add_attribute(attribute),
            Cell::new(entry.is_enabled).add_attribute(attribute),
            Cell::new(format!("{:?}", entry.working_mode)).add_attribute(attribute),
            Cell::new(format!("{}", entry.target_state_of_charge.0)).add_attribute(attribute),
            Cell::new(format!("{}", entry.power.0)).add_attribute(attribute),
        ]);
    }
    table
}
