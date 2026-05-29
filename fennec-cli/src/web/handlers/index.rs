use axum::extract::State;
use maud::{Markup, PreEscaped, html};

use crate::{
    battery::WorkingMode,
    prelude::*,
    quantity::{currency::Mills, energy::WattHours},
    web,
    web::{battery::StateOfCharge, partials, working_mode::WorkingModeColor},
};

#[instrument(skip_all)]
#[expect(clippy::too_many_lines)]
#[expect(clippy::significant_drop_tightening)]
pub async fn get(State(state): State<web::State>) -> Markup {
    info!("access");
    let hunter_state = state.hunter.read().await;
    let energy_profile = state.logger_runner.energy_profile().await;
    let battery_metrics = energy_profile.battery_metrics.as_ref();

    partials::page(
        "Fennec",
        html! {
            section.section.pb-5 {
                div.box {
                    div.field.is-grouped.is-grouped-multiline {
                        div.control {
                            div.tags.has-addons {
                                span.tag.is-info {
                                    span.icon-text {
                                        span.icon { i.fas.fa-money-bill {} }
                                        span { "Loss" }
                                    }
                                }
                                span.tag {
                                    (hunter_state.metrics.losses.total())
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
                                        span { (hunter_state.metrics.internal_battery_flow.import) }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-angle-up {} }
                                        span { (hunter_state.metrics.internal_battery_flow.export) }
                                    }
                                }
                            }
                        }
                        @if let Some(battery_metrics) = battery_metrics {
                            div.control {
                                div.tags.has-addons {
                                    span.tag.is-info {
                                        span.icon-text {
                                            span.icon { i.fas.fa-rotate {} }
                                            span { "Cycles" }
                                        }
                                    }
                                    span.tag {
                                        (format!("{:.1}", (hunter_state.metrics.internal_battery_flow.import + hunter_state.metrics.internal_battery_flow.export) / battery_metrics.actual_capacity() / 2.0))
                                    }
                                }
                            }
                        }
                    }

                    div.field.is-grouped.is-grouped-multiline {
                        div.control {
                            div.tags.has-addons {
                                span.tag.is-info {
                                    span.icon-text {
                                        span.icon { i.fas.fa-plug-circle-bolt {} }
                                        span { "EPS" }
                                    }
                                }
                                span.tag {
                                    (energy_profile.eps_active_power())
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
                                        span { (format!("{:.1}%", 100.0 * hunter_state.battery_efficiency.round_trip())) }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-angle-down {} }
                                        span { (format!("{:.1}%", 100.0 * hunter_state.battery_efficiency.charging)) }
                                    }
                                }
                                span.tag {
                                    span.icon-text {
                                        span.icon { i.fas.fa-angle-up {} }
                                        span { (format!("{:.1}%", 100.0 * hunter_state.battery_efficiency.discharging)) }
                                    }
                                }
                            }
                        }
                        div.control {
                            div.tags.has-addons {
                                span.tag.is-info {
                                    span.icon-text {
                                        span.icon { i.fas.fa-plug-circle-minus {} }
                                        span { "Parasitic load" }
                                    }
                                }
                                span.tag {
                                    (hunter_state.battery_efficiency.parasitic_load)
                                }
                            }
                        }
                    }

                    @if let Some(battery_metrics) = battery_metrics {
                        div.field.is-grouped.is-grouped-multiline {
                            div.control {
                                div.tags.has-addons {
                                     @let state_of_charge = StateOfCharge {
                                        residual_energy: battery_metrics.residual_energy().into(),
                                        actual_capacity: battery_metrics.actual_capacity(),
                                    };
                                    span.tag.(state_of_charge.class()) {
                                        span.icon-text {
                                            (state_of_charge.icon())
                                            span { "Charge" }
                                        }
                                    }
                                    span.tag { (battery_metrics.charge) }
                                    span.tag { (WattHours::from(battery_metrics.residual_energy())) }
                                }
                            }
                            div.control {
                                div.tags.has-addons {
                                    span.tag.is-info {
                                        span.icon-text {
                                            span.icon { i.fas.fa-star-of-life {} }
                                            span { "Health" }
                                        }
                                    }
                                    span.tag { (battery_metrics.health) }
                                    span.tag { (battery_metrics.actual_capacity()) }
                                }
                            }
                        }
                    }
                }
            }

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
                                    @for ((interval, energy_price), step) in &hunter_state.steps {
                                        tr.(WorkingModeColor(step.working_mode)) {
                                            td {
                                                (interval.start().format("%b"))
                                                (PreEscaped("&nbsp;"))
                                                (interval.start().format("%d"))
                                            }
                                            td { (interval.start().format("%H:%M")) }
                                            td { (interval.end().format("%H:%M")) }
                                            td { (step.duration) }
                                            td.has-text-right.has-text-weight-medium[step.working_mode != WorkingMode::Idle] {
                                                (energy_price.import)
                                            }
                                            td {
                                                (step.working_mode)
                                            }
                                            td.has-text-right.has-text-weight-medium[step.energy_balance.grid.import >= WattHours::ONE] {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { (step.energy_balance.grid.import) }
                                                    span.icon { i.fas.fa-angles-down {} }
                                                }
                                            }
                                            td.has-text-right.has-text-weight-medium[step.energy_balance.grid.export >= WattHours::ONE] {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { (step.energy_balance.grid.export) }
                                                    span.icon { i.fas.fa-angles-up {} }
                                                }
                                            }
                                            td.has-text-right.has-text-weight-medium[step.energy_balance.battery.import >= WattHours::ONE] {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { (step.energy_balance.battery.import) }
                                                    span.icon { i.fas.fa-angle-down {} }
                                                }
                                            }
                                            td.has-text-right.has-text-weight-medium[step.energy_balance.battery.export >= WattHours::ONE] {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { (step.energy_balance.battery.export) }
                                                    span.icon { i.fas.fa-angle-up {} }
                                                }
                                            }
                                            td.has-text-right {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { (step.residual_energy_after) }
                                                    @if let Some(battery_metrics) = battery_metrics {
                                                        (StateOfCharge {
                                                            residual_energy: step.residual_energy_after,
                                                            actual_capacity: battery_metrics.actual_capacity(),
                                                        }.icon())
                                                    }
                                                }
                                            }
                                            td.has-text-right.has-text-weight-medium[step.metrics.losses.grid >= Mills::TEN] {
                                                (step.metrics.losses.grid)
                                            }
                                            td.has-text-right.has-text-weight-medium[step.metrics.losses.battery >= Mills::TEN] {
                                                (step.metrics.losses.battery)
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
