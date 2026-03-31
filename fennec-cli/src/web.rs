mod residual_energy;
pub mod state;
mod working_mode;

use std::net::IpAddr;

use axum::{Router, extract::State, routing::get};
use clap::crate_version;
use http::StatusCode;
use maud::{DOCTYPE, Markup, html};

use crate::{
    prelude::*,
    web::{
        residual_energy::ResidualEnergyIconText,
        state::ApplicationState,
        working_mode::WorkingModeColor,
    },
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
async fn get_health(
    State(state): State<ApplicationState>,
) -> Result<StatusCode, (StatusCode, String)> {
    info!("check");
    state.error_message().map_or(Ok(StatusCode::NO_CONTENT), |message| {
        Err((StatusCode::INTERNAL_SERVER_ERROR, message))
    })
}

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
#[expect(clippy::too_many_lines)]
async fn get_index(State(state): State<ApplicationState>) -> Markup {
    info!("access");

    let logger = state.logger.read().unwrap();
    let hunter = state.hunter.read().unwrap();

    let error_message = state.error_message();
    let navbar_class = if error_message.is_some() { "is-danger" } else { "is_success" };

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
                nav.navbar.(navbar_class) role="navigation" aria-label="main navigation" {
                    div.container {
                        div.navbar-brand {
                            a.navbar-item href="/" {
                                svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100" {
                                    text y="1em" font-size="90" { "🦊" }
                                }
                            }
                            div.navbar-item {
                                div {
                                    p.is-size-7 { "version" }
                                    p.is-size-7.is-uppercase.has-text-weight-medium { (crate_version!()) }
                                }
                            }
                            div.navbar-item {
                                div {
                                    p.is-size-7 { "logger" }
                                    p.is-size-7.is-uppercase.has-text-weight-medium { (logger.status()) }
                                }
                            }
                            div.navbar-item {
                                div {
                                    p.is-size-7 { "solver" }
                                    p.is-size-7.is-uppercase.has-text-weight-medium { (hunter.status()) }
                                }
                            }
                        }
                    }
                }
                section.section {
                    div.container {
                        @if let Some(message) = &error_message {
                            article.message.is-danger {
                                div.message-header {
                                    p { "Error" }
                                }
                                div.message-body {
                                    (message)
                                }
                            }
                        }
                        @if let Ok(state) = &hunter.result {
                            div.box {
                                div.table-container {
                                    table.table.is-striped.is-narrow.is-hoverable.is-fullwidth {
                                        thead { (steps_table_header()) }
                                        tfoot { (steps_table_header()) }
                                        tbody {
                                            @for step in &state.steps {
                                                tr.(WorkingModeColor(step.working_mode)) {
                                                    td { (step.interval.start.format("%b %d")) }
                                                    td { (step.interval.start.format("%H:%M")) }
                                                    td { (step.interval.end.format("%H:%M")) }
                                                    td { (step.duration) }
                                                    td.has-text-right.has-text-weight-semibold { (step.energy_price) }
                                                    td { (step.working_mode) }
                                                    td.has-text-right {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span { (step.energy_balance.grid.import) }
                                                            span.icon { i.fas.fa-chevron-down {} }
                                                        }
                                                    }
                                                    td.has-text-right {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span { (step.energy_balance.grid.export) }
                                                            span.icon { i.fas.fa-chevron-up {} }
                                                        }
                                                    }
                                                    td.has-text-right {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span { (step.energy_balance.battery.import) }
                                                            span.icon { i.fas.fa-chevron-down {} }
                                                        }
                                                    }
                                                    td.has-text-right {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            span { (step.energy_balance.battery.export) }
                                                            span.icon { i.fas.fa-chevron-up {} }
                                                        }
                                                    }
                                                    td.has-text-right.has-text-weight-semibold {
                                                        (ResidualEnergyIconText {
                                                            residual_energy: step.residual_energy_after,
                                                            actual_capacity: state.actual_capacity,
                                                        })
                                                    }
                                                    td.has-text-right { (step.metrics.losses.grid) }
                                                    td.has-text-right { (step.metrics.losses.battery) }
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
