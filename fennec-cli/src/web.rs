mod battery;
pub mod state;
mod working_mode;

use std::net::IpAddr;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use chrono_humanize::HumanTime;
use clap::crate_version;
use http::{StatusCode, header};
use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::{
    battery::WorkingMode,
    prelude::*,
    quantity::{currency::Mills, energy::WattHours},
    web::{battery::StateOfCharge, state::ApplicationState, working_mode::WorkingModeColor},
};

pub async fn serve(address: IpAddr, port: u16, state: ApplicationState) -> Result {
    info!(%address, port, "serving web UI…");
    let app = Router::new()
        .route("/", get(get_index))
        .route("/health", get(get_health))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}

#[instrument(skip_all)]
async fn get_health(State(state): State<ApplicationState>) -> impl IntoResponse {
    info!("check");

    #[expect(clippy::option_if_let_else)]
    let body = match state.error_message() {
        Some(message) => Err((StatusCode::INTERNAL_SERVER_ERROR, message)),
        None => Ok(StatusCode::NO_CONTENT),
    };

    ([(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")], body)
}

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
#[expect(clippy::too_many_lines)]
async fn get_index(State(state): State<ApplicationState>) -> Markup {
    info!("access");

    let logger = state.logger.read().unwrap();
    let hunter = state.hunter.read().unwrap();

    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Fennec" }
                link
                    rel="icon"
                    href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='1em' font-size='90'>🦊</text></svg>";
                link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/bulma/1.0.4/css/bulma.min.css"
                    integrity="sha512-yh2RE0wZCVZeysGiqTwDTO/dKelCbS9bP2L94UvOFtl/FKXcNAje3Y2oBg/ZMZ3LS1sicYk4dYVGtDex75fvvA=="
                    crossorigin="anonymous"
                    referrerpolicy="no-referrer";
                link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/7.0.1/css/all.min.css"
                    integrity="sha512-2SwdPD6INVrV/lHTZbO2nodKhrnDdJK9/kg2XD1r9uGqPo1cUbujc+IYdlYdEErWNu69gVcYgdxlmVmzTWnetw=="
                    crossorigin="anonymous"
                    referrerpolicy="no-referrer";
            }
            body {
                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.container {
                        div.navbar-brand {
                            a.navbar-item href="/" {
                                svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100" {
                                    text y="0.95em" font-size="90" { "🦊" }
                                }
                            }
                            span.navbar-item {
                                (crate_version!())
                            }
                        }
                    }
                }
                section.section.pb-5 {
                    div.container {
                        div.field.is-grouped.is-grouped-multiline {
                            div.control {
                                div.tags.has-addons {
                                    span.tag.(if hunter.result.is_ok() { "is-success" } else { "is-error" }) {
                                        span.icon-text {
                                            span.icon { i.fas.fa-timeline {} }
                                            span { "Hunter" }
                                        }
                                    }
                                    span.tag {
                                        (HumanTime::from(hunter.last_run_at))
                                    }
                                }
                            }
                            div.control {
                                div.tags.has-addons {
                                    span.tag.(if logger.result.is_ok() { "is-success" } else { "is-error" }) {
                                        span.icon-text {
                                            span.icon { i.fas.fa-heart-circle-bolt {} }
                                            span { "Logger" }
                                        }
                                    }
                                    span.tag {
                                        (HumanTime::from(logger.last_run_at))
                                    }
                                }
                            }
                        }

                        @if let Ok(hunter_state) = &hunter.result {
                            div.field.is-grouped.is-grouped-multiline {
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-info {
                                            span.icon-text {
                                                span.icon { i.fas.fa-money-bill {} }
                                                span { "Profit" }
                                            }
                                        }
                                        span.tag {
                                            (hunter_state.profit())
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
                                @if let Ok(logger_state) = &logger.result {
                                    div.control {
                                        div.tags.has-addons {
                                            span.tag.is-info {
                                                span.icon-text {
                                                    span.icon { i.fas.fa-rotate {} }
                                                    span { "Cycles" }
                                                }
                                            }
                                            span.tag {
                                                (format!("{:.1}", (hunter_state.metrics.internal_battery_flow.import + hunter_state.metrics.internal_battery_flow.export) / logger_state.battery.actual_capacity() / 2.0))
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
                                            (hunter_state.energy_profile.average_eps_power)
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
                                                span { (format!("{:.1}%", 100.0 * hunter_state.energy_profile.battery_efficiency.round_trip())) }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angle-down {} }
                                                span { (format!("{:.1}%", 100.0 * hunter_state.energy_profile.battery_efficiency.charging)) }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angle-up {} }
                                                span { (format!("{:.1}%", 100.0 * hunter_state.energy_profile.battery_efficiency.discharging)) }
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
                                            (hunter_state.energy_profile.battery_efficiency.parasitic_load)
                                        }
                                    }
                                }
                            }
                        }

                        @if let Ok(logger_state) = &logger.result {
                            div.field.is-grouped.is-grouped-multiline {
                                div.control {
                                    div.tags.has-addons {
                                         @let state_of_charge = StateOfCharge {
                                            residual_energy: logger_state.battery.residual_energy(),
                                            actual_capacity: Some(logger_state.battery.actual_capacity()),
                                        };
                                        span.tag.(state_of_charge.class()) {
                                            span.icon-text {
                                                (state_of_charge.icon())
                                                span { "Charge" }
                                            }
                                        }
                                        span.tag { (logger_state.battery.charge) }
                                        span.tag { (logger_state.battery.residual_energy()) }
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
                                        span.tag { (logger_state.battery.health) }
                                        span.tag { (logger_state.battery.actual_capacity()) }
                                    }
                                }
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-info {
                                            span.icon-text {
                                                span.icon { i.fas.fa-gear {} }
                                                span { "Min SoC" }
                                            }
                                        }
                                        span.tag { (logger_state.battery.min_system_charge) }
                                    }
                                }
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-info {
                                            span.icon-text {
                                                span.icon { i.fas.fa-user-gear {} }
                                                span { "SoC range" }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-greater-than-equal {} }
                                                span { (logger_state.battery.charge_range.min) }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-less-than-equal {} }
                                                span { (logger_state.battery.charge_range.max) }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        @if let Some(message) = state.error_message() {
                            article.message.is-danger {
                                div.message-header {
                                    p { "Error" }
                                }
                                div.message-body {
                                    (message)
                                }
                            }
                        }
                    }
                }

                section.section.pt-5 {
                    div.container {
                        @if let Ok(hunter_state) = &hunter.result {
                            div.card {
                                header.card-header {
                                    p.card-header-title {
                                        span.icon-text {
                                            span.icon { i.fas.fa-calendar {} }
                                            span { "Schedule" }
                                        }
                                    }
                                }
                                div.card-content {
                                    div.table-container {
                                        table.table.is-striped.is-narrow.is-hoverable.is-fullwidth {
                                            thead { (steps_table_header()) }
                                            tfoot { (steps_table_header()) }
                                            tbody {
                                                @for step in &hunter_state.steps {
                                                    tr.(WorkingModeColor(step.working_mode)) {
                                                        td {
                                                            (step.interval.start.format("%b"))
                                                            (PreEscaped("&nbsp;"))
                                                            (step.interval.start.format("%d"))
                                                        }
                                                        td { (step.interval.start.format("%H:%M")) }
                                                        td { (step.interval.end.format("%H:%M")) }
                                                        td { (step.duration) }
                                                        td.has-text-right.has-text-weight-medium[step.working_mode != WorkingMode::Idle] {
                                                            (step.energy_price)
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
                                                                (StateOfCharge {
                                                                    residual_energy: step.residual_energy_after,
                                                                    actual_capacity: logger.result.as_ref().ok().map(|state| state.battery.actual_capacity()),
                                                                }.icon())
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
                    }
                }
            }
        }
    }
}

fn steps_table_header() -> Markup {
    html! {
        tr {
            th { "Date" }
            th { "Start" br; "time" }
            th { "End" br; "time" }
            th { "Duration" }
            th.has-text-right { "Energy" br; "price" }
            th { "Working" br; "mode" }
            th.has-text-right { "Grid" br; "import" }
            th.has-text-right { "Grid" br; "export" }
            th.has-text-right { "Battery" br; "import" }
            th.has-text-right { "Battery" br; "export" }
            th.has-text-right { "Residual" br; "after" }
            th.has-text-right { "Grid" br; "loss" }
            th.has-text-right { "Battery" br; "loss" }
        }
    }
}
