pub mod application;
mod battery;
pub mod colors;
mod working_mode;

use std::net::IpAddr;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use chrono::NaiveTime;
use clap::crate_version;
use http::{StatusCode, header};
use itertools::Itertools;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use plotters::{
    backend::SVGBackend,
    chart::ChartBuilder,
    drawing::IntoDrawingArea,
    series::LineSeries,
    style,
    style::{Color, full_palette},
};

use crate::{
    battery::WorkingMode,
    energy,
    prelude::*,
    quantity::{currency::Mills, energy::WattHours},
    web::{battery::StateOfCharge, working_mode::WorkingModeColor},
};

pub async fn serve(address: IpAddr, port: u16, state: application::State) -> Result {
    info!(%address, port, "serving web UI…");
    let app = Router::new()
        .route("/", get(get_index))
        .route("/readiness", get(get_readiness))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}

#[instrument(skip_all)]
async fn get_readiness() -> impl IntoResponse {
    info!("check");
    (StatusCode::NO_CONTENT, [(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")])
}

#[instrument(skip_all)]
#[expect(clippy::too_many_lines)]
#[expect(clippy::significant_drop_tightening)]
async fn get_index(State(state): State<application::State>) -> Markup {
    info!("access");

    let logger = state.logger.read().unwrap();
    let logger_state = &logger;
    let mean_balance = logger_state.energy_profile.mean_balance();

    let hunter = state.hunter.read().unwrap();
    let hunter_state = &hunter;

    html! {
        (DOCTYPE)
        html lang="en-GB" {
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
                style {
                    ".has-plotters-fix svg { width: 100%; height: auto; display: block; }"
                }
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
                                        (hunter_state.average_eps_power)
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

                        div.field.is-grouped.is-grouped-multiline {
                            div.control {
                                div.tags.has-addons {
                                     @let state_of_charge = StateOfCharge {
                                        residual_energy: logger_state.battery.residual_energy(),
                                        actual_capacity: logger_state.battery.actual_capacity(),
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
                        }
                    }
                }

                section.section.pt-5 {
                    div.card {
                        header.card-header {
                            p.card-header-title {
                                span.icon-text {
                                    span.icon { i.fa-solid.fa-calendar {} }
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
                                                        (StateOfCharge {
                                                            residual_energy: step.residual_energy_after,
                                                            actual_capacity: logger_state.battery.actual_capacity(),
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

                section.section.pt-0 {
                    div.card {
                        header.card-header {
                            p.card-header-title {
                                span.icon-text {
                                    span.icon { i.fa-solid.fa-chart-line {} }
                                    span { "Energy profile" }
                                }
                            }
                        }
                        div.card-content {
                            div.field.is-grouped.is-grouped-multiline {
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-success {
                                            span.icon-text {
                                                span.icon { i.fas.fa-charging-station {} }
                                                span { "Battery mean import" }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angle-down {} }
                                                span { (mean_balance.battery.import) }
                                            }
                                        }
                                    }
                                }
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-warning {
                                            span.icon-text {
                                                span.icon { i.fas.fa-charging-station {} }
                                                span { "Battery mean export" }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angle-up {} }
                                                span { (mean_balance.battery.export) }
                                            }
                                        }
                                    }
                                }
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-danger {
                                            span.icon-text {
                                                span.icon { i.fas.fa-plug {} }
                                                span { "Grid mean import" }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angles-down {} }
                                                span { (mean_balance.grid.import) }
                                            }
                                        }
                                    }
                                }
                                div.control {
                                    div.tags.has-addons {
                                        span.tag.is-link {
                                            span.icon-text {
                                                span.icon { i.fas.fa-plug {} }
                                                span { "Grid mean export" }
                                            }
                                        }
                                        span.tag {
                                            span.icon-text {
                                                span.icon { i.fas.fa-angles-up {} }
                                                span { (mean_balance.grid.export) }
                                            }
                                        }
                                    }
                                }
                            }

                            figure.image.has-plotters-fix {
                                (energy_profile_chart(&logger_state.energy_profile))
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

#[must_use]
fn energy_profile_chart(energy_profile: &energy::NewProfile) -> Markup {
    let mut points = {
        let mean_balance = energy_profile.mean_balance();
        (0..24)
            .cartesian_product([0, 10, 20, 30, 40, 50])
            .map(|(hour, minute)| {
                (
                    f64::from(hour) + f64::from(minute) / 60.0,
                    NaiveTime::from_hms_opt(hour % 24, minute, 0).unwrap(),
                )
            })
            .map(|(x, naive_time)| (x, mean_balance + energy_profile.deviation_at(naive_time)))
            .collect_vec()
    };
    let (min_y, max_y) = {
        let values = points
            .iter()
            .flat_map(|(_, balance)| {
                [
                    balance.grid.import,
                    balance.grid.export,
                    balance.battery.import,
                    balance.battery.export,
                ]
            })
            .map(|power| power.0);
        (
            values.clone().min_by(f64::total_cmp).unwrap_or_default(),
            values.max_by(f64::total_cmp).unwrap_or_default(),
        )
    };
    points.push((24.0, points[0].1));

    let mut svg = PreEscaped(String::new());
    {
        let drawing_area = SVGBackend::with_string(&mut svg.0, (1000, 250)).into_drawing_area();
        let mut chart = ChartBuilder::on(&drawing_area)
            .x_label_area_size(20)
            .y_label_area_size(40)
            .margin_top(10)
            .build_cartesian_2d(0_f64..24_f64, min_y..max_y)
            .unwrap();
        chart
            .configure_mesh()
            .bold_line_style(&full_palette::GREY_600)
            .light_line_style(&full_palette::GREY_600.mix(0.25))
            .label_style(&full_palette::GREY_600)
            .y_max_light_lines(0)
            .draw()
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.grid.import.0)),
                colors::DANGER.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.grid.export.0)),
                colors::LINK.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.battery.import.0)),
                colors::SUCCESS.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.battery.export.0)),
                colors::WARNING.stroke_width(2),
            ))
            .unwrap();
        drawing_area.present().unwrap();
    }
    svg
}
