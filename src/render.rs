use comfy_table::{Cell, Color, Table, modifiers, presets};

use crate::{
    api::foxess::{TimeSlotSequence, WorkingMode as FoxEssWorkingMode},
    core::{
        metrics::Metrics,
        series::Series,
        solution::Step,
        working_mode::WorkingMode as CoreWorkingMode,
    },
    prelude::*,
    units::currency::Cost,
};

pub fn try_render_steps(metrics: &Series<Metrics>, steps: &Series<Step>) -> Result<Table> {
    ensure!(!metrics.is_empty());
    #[allow(clippy::cast_precision_loss)]
    let average_rate = metrics.into_iter().map(|point| point.value.grid_rate.0).sum::<f64>()
        / metrics.len() as f64;

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED).apply_modifier(modifiers::UTF8_ROUND_CORNERS);
    table.set_header(vec![
        "Time",
        "Grid rate\n€/kWh",
        "Solar\nW/m²",
        "Before\nkWh",
        "Mode",
        "After\nkWh",
        "Grid usage\nkWh",
        "Loss\n€",
    ]);
    for point in metrics.try_zip(steps) {
        let point = point?;
        let (metrics, step) = point.value;
        let solar_color = match metrics.solar_power_density {
            Some(density) if density.0 > 0.5 => Color::Green,
            Some(density) if density.0 > 0.25 => Color::DarkYellow,
            Some(_) => Color::Red,
            _ => Color::Reset,
        };
        let solar_content = metrics
            .solar_power_density
            .map_or_else(|| "unknown".to_string(), |value| format!("{:.0}", value.0 * 1000.0));
        table.add_row(vec![
            Cell::new(point.time.format("%H:%M").to_string()),
            Cell::new(format!("{:.2}", metrics.grid_rate))
                .fg(if metrics.grid_rate.0 >= average_rate { Color::Red } else { Color::Green }),
            Cell::new(solar_content).fg(solar_color),
            Cell::new(format!("{:.2}", step.residual_energy_before)),
            Cell::new(format!("{:?}", step.working_mode)).fg(match step.working_mode {
                CoreWorkingMode::Charging => Color::Green,
                CoreWorkingMode::Discharging => Color::Red,
                CoreWorkingMode::Balancing => Color::DarkYellow,
                CoreWorkingMode::Idle => Color::Reset,
            }),
            Cell::new(format!("{:.2}", step.residual_energy_after)),
            Cell::new(format!("{:+.2}", step.grid_consumption)),
            Cell::new(format!("{:+.2}", step.loss)).fg(if step.loss > Cost::ZERO {
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
    table.set_header(vec!["Start", "End", "Mode", "Power, W"]);
    for time_slot in sequence {
        let mode_color = match time_slot.working_mode {
            FoxEssWorkingMode::ForceDischarge if time_slot.feed_power_watts != 0 => Color::Red,
            FoxEssWorkingMode::ForceCharge if time_slot.feed_power_watts != 0 => Color::Green,
            FoxEssWorkingMode::SelfUse => Color::DarkYellow,
            _ => Color::White,
        };
        table.add_row(vec![
            Cell::new(time_slot.start_time.to_string()),
            Cell::new(time_slot.end_time.to_string()),
            Cell::new(format!("{:?}", time_slot.working_mode)).fg(mode_color),
            Cell::new(time_slot.feed_power_watts.to_string()),
        ]);
    }
    table
}
