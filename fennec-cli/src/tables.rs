use average::Mean;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    core::{step::Step, working_mode::WorkingMode},
    quantity::{currency::Mills, energy::WattHours, rate::KilowattHourRate},
};

pub fn build_steps_table(steps: &[Step]) -> Table {
    let mean_rate: KilowattHourRate = {
        let estimate: Mean = steps.iter().map(|step| step.grid_rate.0).collect();
        if estimate.is_empty() {
            KilowattHourRate::zero()
        } else {
            KilowattHourRate(estimate.mean())
        }
    };

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL_CONDENSED)
        .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
        .enforce_styling()
        .set_header(vec![
            "Date",
            "Start",
            "End",
            "Rate",
            "Mode",
            "Grid ↓",
            "Grid ↑",
            "Battery ↓",
            "Battery ↑",
            "Residual",
            "Loss",
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
            Cell::new(step.working_mode).fg(step.working_mode.color()),
            Cell::new(step.system_flow.grid.import).set_alignment(CellAlignment::Right).fg(
                if step.system_flow.grid.import > WattHours::ONE {
                    Color::Red
                } else {
                    Color::Green
                },
            ),
            Cell::new(step.system_flow.grid.export).set_alignment(CellAlignment::Right).fg(
                if step.system_flow.grid.export > WattHours::ONE {
                    Color::Blue
                } else {
                    Color::Reset
                },
            ),
            Cell::new(step.system_flow.battery.import)
                .fg(if step.system_flow.battery.import > WattHours::ONE {
                    WorkingMode::Charge.color()
                } else {
                    Color::Reset
                })
                .set_alignment(CellAlignment::Right),
            Cell::new(step.system_flow.battery.export)
                .fg(if step.system_flow.battery.export > WattHours::ONE {
                    WorkingMode::Discharge.color()
                } else {
                    Color::Reset
                })
                .set_alignment(CellAlignment::Right),
            Cell::new(step.residual_energy_after).set_alignment(CellAlignment::Right),
            Cell::new(step.loss)
                .set_alignment(CellAlignment::Right)
                .fg(if step.loss >= Mills::TEN { Color::Red } else { Color::Green }),
        ]);
    }
    table
}
