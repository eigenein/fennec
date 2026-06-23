use std::sync::Arc;

use axum::extract::State;
use maud::{Markup, PreEscaped, html};
use tokio::sync::RwLock;

use crate::{
    battery::WorkingMode,
    prelude::*,
    quantity::{currency::Mills, energy::WattHours},
    web::{partials, working_mode::WorkingModeColor},
};

#[instrument(skip_all)]
#[expect(clippy::too_many_lines)]
#[expect(clippy::significant_drop_tightening)]
pub async fn get(State(state): State<Arc<RwLock<crate::State>>>) -> Markup {
    debug!("access");
    let state = state.read().await;
    let backtrack = state.backtrack.as_ref();
    let energy_profile = &state.energy_profile;
    let battery_tracker = energy_profile.battery.tracker.as_ref();

    partials::page(
        "Fennec",
        html! {
            section.section.pb-5 {
                div.box {
                    div.field.is-grouped.is-grouped-multiline {
                        @if let Some(backtrack) = backtrack {
                            div.control {
                                div.tags.has-addons {
                                    span.tag.is-info {
                                        span.icon-text {
                                            span.icon { i.fas.fa-money-bill {} }
                                            span { "Loss" }
                                        }
                                    }
                                    span.tag {
                                        (backtrack.metrics.losses.total())
                                    }
                                }
                            }
                            div.control {
                                div.tags.has-addons {
                                    span.tag.is-info {
                                        span.icon-text {
                                            span.icon { i.fas.fa-arrow-right-arrow-left {} }
                                            span { "Flow" }
                                        }
                                    }
                                    span.tag {
                                        span.icon-text {
                                            span.icon { i.fas.fa-angle-down {} }
                                            span { (backtrack.metrics.internal_battery_flow.import) }
                                        }
                                    }
                                    span.tag {
                                        span.icon-text {
                                            span.icon { i.fas.fa-angle-up {} }
                                            span { (backtrack.metrics.internal_battery_flow.export) }
                                        }
                                    }
                                }
                            }
                        }
                        @if let Some(battery_tracker) = battery_tracker {
                            div.control {
                                div.tags.has-addons {
                                    span.tag.is-info {
                                        span.icon-text {
                                            span.icon { i.fa-solid.fa-battery-half {} }
                                            span { "Charge" }
                                        }
                                    }
                                    span.tag { (WattHours::from(battery_tracker.residual_energy)) }
                                }
                            }
                        }
                        div.control {
                            div.tags.has-addons {
                                span.tag.is-info {
                                    span.icon-text {
                                        span.icon { i.fas.fa-charging-station {} }
                                        span { "Efficiency" }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-rotate {} }
                                        span { (format!("{:.1}%", 100.0 * energy_profile.battery.efficiency.round_trip())) }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-angle-down {} }
                                        span { (format!("{:.1}%", 100.0 * energy_profile.battery.efficiency.import)) }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-angle-up {} }
                                        span { (format!("{:.1}%", 100.0 * energy_profile.battery.efficiency.export)) }
                                    }
                                }
                            }
                        }
                        div.control {
                            div.tags.has-addons {
                                span.tag.is-info {
                                    span.icon-text {
                                        span.icon { i.fas.fa-plug-circle-bolt {} }
                                        span { "EPS" }
                                    }
                                }
                                span.tag {
                                    (energy_profile.balance.eps_active_power.0)
                                }
                            }
                        }
                    }
                }
            }

            @if let Some(backtrack) = backtrack {
                section.section.py-5 {
                    div.card {
                        header.card-header {
                            p.card-header-title { "Schedule" }
                        }
                        div.card-content {
                            div.table-container {
                                table.table.is-striped.is-narrow.is-hoverable.is-fullwidth {
                                    thead { (steps_table_header()) }
                                    tfoot { (steps_table_header()) }
                                    tbody {
                                        @for slot in backtrack.schedule.iter() {
                                            tr.(WorkingModeColor(slot.value.1.working_mode)) {
                                                td {
                                                    (slot.interval.start().format("%b"))
                                                    (PreEscaped("&nbsp;"))
                                                    (slot.interval.start().format("%d"))
                                                }
                                                td { (slot.interval.start().format("%H:%M")) }
                                                td { (slot.interval.end().format("%H:%M")) }
                                                td { (slot.value.1.duration) }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.working_mode != WorkingMode::Idle] {
                                                    (slot.value.0.import)
                                                }
                                                td {
                                                    (slot.value.1.working_mode)
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.energy_balance.grid.import >= WattHours::ONE] {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { (slot.value.1.energy_balance.grid.import) }
                                                        span.icon { i.fas.fa-angles-down {} }
                                                    }
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.energy_balance.grid.export >= WattHours::ONE] {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { (slot.value.1.energy_balance.grid.export) }
                                                        span.icon { i.fas.fa-angles-up {} }
                                                    }
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.energy_balance.battery.import >= WattHours::ONE] {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { (slot.value.1.energy_balance.battery.import) }
                                                        span.icon { i.fas.fa-angle-down {} }
                                                    }
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.energy_balance.battery.export >= WattHours::ONE] {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span { (slot.value.1.energy_balance.battery.export) }
                                                        span.icon { i.fas.fa-angle-up {} }
                                                    }
                                                }
                                                td.has-text-right {
                                                    (slot.value.1.energy_level_after)
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.metrics.losses.grid >= Mills::TEN] {
                                                    (slot.value.1.metrics.losses.grid)
                                                }
                                                td.has-text-right.has-text-weight-medium[slot.value.1.metrics.losses.battery >= Mills::TEN] {
                                                    (slot.value.1.metrics.losses.battery)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn steps_table_header() -> Markup {
    html! {
        tr {
            th { "Date" }
            th { "Start time" }
            th { "End time" }
            th { "Duration" }
            th.has-text-right { "Energy price" }
            th { "Working mode" }
            th.has-text-right { "Grid import" }
            th.has-text-right { "Grid export" }
            th.has-text-right { "Battery import" }
            th.has-text-right { "Battery export" }
            th.has-text-right { "Residual after" }
            th.has-text-right { "Grid loss" }
            th.has-text-right { "Battery loss" }
        }
    }
}
