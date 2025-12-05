use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, modifiers, presets};

use crate::{
    api::foxess::{TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
    cli::BatteryArgs,
    core::{series::Aggregate, solver::step::Step, working_mode::WorkingMode as CoreWorkingMode},
    quantity::{
        cost::Cost,
        energy::KilowattHours,
        power::{Kilowatts, Watts},
        rate::KilowattHourRate,
    },
};

pub fn build_steps_table(
    steps: &[Step],
    battery_args: BatteryArgs,
    capacity: KilowattHours,
) -> Table {
    let median_rate =
        steps.iter().map(|step| step.grid_rate).median().unwrap_or(KilowattHourRate::ZERO);
    let min_residual_energy = capacity * (f64::from(battery_args.min_soc_percent) / 100.0);

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED).apply_modifier(modifiers::UTF8_ROUND_CORNERS);
    table.enforce_styling();
    table.set_header(vec![
        "Start",
        "End",
        "Grid rate",
        "Stand-by",
        "Mode",
        "Before",
        "After",
        "Grid usage",
        "Loss",
    ]);
    for step in steps {
        table.add_row(vec![
            Cell::new(step.interval.start.format("%H:%M")),
            Cell::new(step.interval.end.format("%H:%M")).add_attribute(Attribute::Dim),
            Cell::new(step.grid_rate).fg(if step.grid_rate >= median_rate {
                Color::Red
            } else {
                Color::Green
            }),
            Cell::new(step.stand_by_power).set_alignment(CellAlignment::Right).fg(
                if step.stand_by_power <= Kilowatts::ZERO {
                    Color::Green
                } else if step.stand_by_power <= battery_args.discharging_power {
                    Color::DarkYellow
                } else {
                    Color::Red
                },
            ),
            Cell::new(format!("{:?}", step.working_mode)).fg(match step.working_mode {
                CoreWorkingMode::Charge => Color::Green,
                CoreWorkingMode::Discharge => Color::Red,
                CoreWorkingMode::Balance => Color::DarkYellow,
                CoreWorkingMode::Backup => Color::Magenta,
                CoreWorkingMode::Idle => Color::Reset,
            }),
            Cell::new(step.residual_energy_before).set_alignment(CellAlignment::Right).fg(
                if step.residual_energy_before > min_residual_energy {
                    Color::Reset
                } else {
                    Color::Red
                },
            ),
            Cell::new(step.residual_energy_after).set_alignment(CellAlignment::Right).fg(
                if step.residual_energy_after > min_residual_energy {
                    Color::Reset
                } else {
                    Color::Red
                },
            ),
            Cell::new(step.grid_consumption).set_alignment(CellAlignment::Right),
            Cell::new(step.loss)
                .set_alignment(CellAlignment::Right)
                .fg(if step.loss >= Cost::ONE_CENT { Color::Red } else { Color::Green }),
        ]);
    }
    table
}

#[must_use]
pub fn build_time_slot_sequence_table(sequence: &TimeSlotSequence) -> Table {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED).apply_modifier(modifiers::UTF8_ROUND_CORNERS);
    table.enforce_styling();
    table.set_header(vec!["Start", "End", "Mode", "Feed power"]);
    for time_slot in sequence {
        let mode_color = match time_slot.working_mode {
            FoxEssWorkingMode::ForceDischarge if time_slot.feed_power != Watts(0) => Color::Red,
            FoxEssWorkingMode::ForceCharge if time_slot.feed_power != Watts(0) => Color::Green,
            FoxEssWorkingMode::SelfUse => Color::DarkYellow,
            FoxEssWorkingMode::BackUp => Color::Magenta,
            _ => Color::Reset,
        };
        table.add_row(vec![
            Cell::new(&time_slot.start_time),
            Cell::new(&time_slot.end_time),
            Cell::new(format!("{:?}", time_slot.working_mode)).fg(mode_color),
            Cell::new(time_slot.feed_power).set_alignment(CellAlignment::Right),
        ]);
    }
    table
}
