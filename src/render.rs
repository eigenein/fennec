use comfy_table::{Cell, Color, Table, modifiers, presets};

use crate::{
    api::foxess::{TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
    core::{series::Series, solver::step::Step, working_mode::WorkingMode as CoreWorkingMode},
    prelude::*,
    quantity::{currency::Cost, rate::KilowattHourRate},
};

pub fn try_render_steps(
    grid_rates: &Series<KilowattHourRate>,
    steps: &Series<Step>,
) -> Result<Table> {
    // TODO: extract to a method in `Series`:
    #[allow(clippy::cast_precision_loss)]
    let average_rate =
        grid_rates.iter().map(|(_, grid_rate)| grid_rate.0).sum::<f64>() / grid_rates.len() as f64;

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED).apply_modifier(modifiers::UTF8_ROUND_CORNERS);
    table.enforce_styling();
    table.set_header(vec![
        "Time",
        "Grid rate",
        "Stand-by",
        "Mode",
        "Before",
        "After",
        "Grid usage",
        "Loss",
    ]);
    for ((time, grid_rate), (right_time, step)) in grid_rates.iter().zip(steps) {
        ensure!(time == right_time);
        // TODO: extract all formatting into `impl Display` for the quantity:
        table.add_row(vec![
            Cell::new(time.format("%H:%M").to_string()),
            Cell::new(format!("{grid_rate:.2} €/kWh")).fg(if grid_rate.0 >= average_rate {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(format!("{:.2} kW", step.stand_by_power)),
            Cell::new(format!("{:?}", step.working_mode)).fg(match step.working_mode {
                CoreWorkingMode::Charging => Color::Green,
                CoreWorkingMode::Discharging => Color::Red,
                CoreWorkingMode::Balancing => Color::DarkYellow,
                CoreWorkingMode::Idle => Color::Reset,
            }),
            Cell::new(format!("{:.2} kWh", step.residual_energy_before)),
            Cell::new(format!("{:.2} kWh", step.residual_energy_after)),
            Cell::new(format!("{:+.2} kWh", step.grid_consumption)),
            Cell::new(format!("{:+.2} €", step.loss)).fg(if step.loss > Cost::ZERO {
                Color::Red
            } else {
                Color::Green
            }),
        ]);
    }
    Ok(table)
}

#[must_use]
pub fn render_time_slot_sequence(sequence: &TimeSlotSequence) -> Table {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED).apply_modifier(modifiers::UTF8_ROUND_CORNERS);
    table.enforce_styling();
    table.set_header(vec!["Start", "End", "Mode", "Power"]);
    for time_slot in sequence {
        let mode_color = match time_slot.working_mode {
            FoxEssWorkingMode::ForceDischarge if time_slot.feed_power_watts != 0 => Color::Red,
            FoxEssWorkingMode::ForceCharge if time_slot.feed_power_watts != 0 => Color::Green,
            FoxEssWorkingMode::SelfUse => Color::DarkYellow,
            _ => Color::Reset,
        };
        table.add_row(vec![
            Cell::new(time_slot.start_time.to_string()),
            Cell::new(time_slot.end_time.to_string()),
            Cell::new(format!("{:?}", time_slot.working_mode)).fg(mode_color),
            Cell::new(format!("{:>4} W", time_slot.feed_power_watts)),
        ]);
    }
    table
}
