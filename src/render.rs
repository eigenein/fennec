use std::ops::Range;

use chrono::{DateTime, Local};
use comfy_table::{Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    api::foxess::{TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
    cli::BatteryArgs,
    core::{series::Point, solver::step::Step, working_mode::WorkingMode as CoreWorkingMode},
    prelude::*,
    quantity::{cost::Cost, energy::KilowattHours, power::Watts, rate::KilowattHourRate},
};

pub fn try_render_steps(
    grid_rates: &[Point<Range<DateTime<Local>>, KilowattHourRate>],
    steps: &[Point<DateTime<Local>, Step>],
    battery_args: BatteryArgs,
    capacity: KilowattHours,
) -> Result<Table> {
    #[allow(clippy::cast_precision_loss)]
    let average_rate =
        grid_rates.iter().map(|(_, grid_rate)| grid_rate.0).sum::<f64>() / grid_rates.len() as f64;

    let min_residual_energy = capacity * (f64::from(battery_args.min_soc_percent) / 100.0);

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
        ensure!(time.start == *right_time);
        table.add_row(vec![
            Cell::new(time.start.format("%H:%M").to_string()),
            Cell::new(grid_rate.to_string()).fg(if grid_rate.0 >= average_rate {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(step.stand_by_power.to_string()).set_alignment(CellAlignment::Right).fg(
                if step.stand_by_power <= -battery_args.charging_power {
                    Color::Green
                } else if step.stand_by_power <= battery_args.discharging_power {
                    Color::DarkYellow
                } else {
                    Color::Red
                },
            ),
            Cell::new(format!("{:?}", step.working_mode)).fg(match step.working_mode {
                CoreWorkingMode::Charging => Color::Green,
                CoreWorkingMode::Discharging => Color::Red,
                CoreWorkingMode::Balancing => Color::DarkYellow,
                CoreWorkingMode::BackupSolar => Color::DarkGreen,
                CoreWorkingMode::Idle => Color::Reset,
            }),
            Cell::new(step.residual_energy_before.to_string())
                .set_alignment(CellAlignment::Right)
                .fg(if step.residual_energy_before > min_residual_energy {
                    Color::Reset
                } else {
                    Color::Red
                }),
            Cell::new(step.residual_energy_after.to_string())
                .set_alignment(CellAlignment::Right)
                .fg(if step.residual_energy_after > min_residual_energy {
                    Color::Reset
                } else {
                    Color::Red
                }),
            Cell::new(step.grid_consumption.to_string()).set_alignment(CellAlignment::Right),
            Cell::new(step.loss.to_string()).fg(if step.loss >= Cost::ONE_CENT {
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
    table.set_header(vec!["Start", "End", "Mode", "Feed power"]);
    for time_slot in sequence {
        let mode_color = match time_slot.working_mode {
            FoxEssWorkingMode::ForceDischarge if time_slot.feed_power != Watts(0) => Color::Red,
            FoxEssWorkingMode::ForceCharge if time_slot.feed_power != Watts(0) => Color::Green,
            FoxEssWorkingMode::SelfUse => Color::DarkYellow,
            _ => Color::Reset,
        };
        table.add_row(vec![
            Cell::new(time_slot.start_time.to_string()),
            Cell::new(time_slot.end_time.to_string()),
            Cell::new(format!("{:?}", time_slot.working_mode)).fg(mode_color),
            Cell::new(time_slot.feed_power.to_string()).set_alignment(CellAlignment::Right),
        ]);
    }
    table
}
