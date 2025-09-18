use comfy_table::{Table, modifiers, presets};

use crate::{
    api::foxess::TimeSlotSequence,
    core::{metrics::Metrics, series::Series, solution::Step},
    prelude::*,
};

pub fn try_render_steps(metrics: &Series<Metrics>, steps: &Series<Step>) -> Result<Table> {
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
        table.add_row(vec![
            point.time.format("%H:%M").to_string(),
            format!("{:.2}", metrics.grid_rate),
            metrics
                .solar_power_density
                .map_or_else(|| "unknown".to_string(), |value| format!("{:.0}", value.0 * 1000.0)),
            format!("{:.2}", step.residual_energy_before),
            format!("{:?}", step.working_mode),
            format!("{:.2}", step.residual_energy_after),
            format!("{:+.2}", step.grid_consumption),
            format!("{:+.2}", step.loss),
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
        table.add_row(vec![
            time_slot.start_time.to_string(),
            time_slot.end_time.to_string(),
            format!("{:?}", time_slot.working_mode),
            time_slot.feed_power_watts.to_string(),
        ]);
    }
    table
}
