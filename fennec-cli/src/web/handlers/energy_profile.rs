use std::ops::Range;

use axum::extract::State;
use chrono::NaiveTime;
use itertools::Itertools;
use maud::{Markup, PreEscaped, html};
use plotters::{
    backend::SVGBackend,
    chart::ChartBuilder,
    coord::{Shift, types::RangedCoordf64},
    prelude::*,
};

use crate::{
    energy,
    prelude::*,
    web::{application, partials},
};

pub const PATH: &str = "/energy-profile";

#[instrument(skip_all)]
#[expect(clippy::significant_drop_tightening)]
pub async fn get(State(state): State<application::State>) -> Markup {
    info!("access");

    let logger = state.logger.read().unwrap();
    let logger_state = &logger;
    let mean_balance = logger_state.energy_profile.mean_balance();

    partials::page(
        "Energy profile",
        html! {
            section.section.pb-5 {
                div.card {
                    header.card-header {
                        p.card-header-title { "Mean balance" }
                    }
                    div.card-content {
                        nav.level.is-mobile {
                            div.level-item.has-text-centered {
                                div {
                                    p.heading {
                                        span.icon-text {
                                            span.icon { i.fa-solid.fa-angle-down {} }
                                            span { "Battery" }
                                        }
                                    }
                                    p.title.has-text-success { (mean_balance.battery.import) }
                                }
                            }
                            div.level-item.has-text-centered {
                                div {
                                    p.heading {
                                        span.icon-text {
                                            span.icon { i.fa-solid.fa-angle-up {} }
                                            span { "Battery" }
                                        }
                                    }
                                    p.title.has-text-warning { (mean_balance.battery.export) }
                                }
                            }
                            div.level-item.has-text-centered {
                                div {
                                    p.heading {
                                        span.icon-text {
                                            span.icon { i.fa-solid.fa-angles-down {} }
                                            span { "Grid" }
                                        }
                                    }
                                    p.title.has-text-danger { (mean_balance.grid.import) }
                                }
                            }
                            div.level-item.has-text-centered {
                                div {
                                    p.heading {
                                        span.icon-text {
                                            span.icon { i.fa-solid.fa-angles-up {} }
                                            span { "Grid" }
                                        }
                                    }
                                    p.title.has-text-link { (mean_balance.grid.export) }
                                }
                            }
                        }
                    }
                }
            }

            section.section.pt-5 {
                div.card {
                    header.card-header {
                        p.card-header-title { "Instant balance" }
                    }
                    div.card-content {
                        figure.image.has-plotters-fix {
                            (energy_balance_chart(&logger_state.energy_profile))
                        }
                    }
                }
            }
        },
    )
}

#[must_use]
fn new_chart(
    buf: &mut Markup,
    value_range: Range<f64>,
) -> (
    DrawingArea<SVGBackend<'_>, Shift>,
    ChartContext<'_, SVGBackend<'_>, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
) {
    let drawing_area = SVGBackend::with_string(&mut buf.0, (1000, 250)).into_drawing_area();
    let mut chart = ChartBuilder::on(&drawing_area)
        .x_label_area_size(20)
        .y_label_area_size(40)
        .margin_top(10)
        .build_cartesian_2d(0_f64..24_f64, value_range)
        .unwrap();
    chart
        .configure_mesh()
        .bold_line_style(full_palette::GREY_600.mix(0.75))
        .light_line_style(full_palette::GREY_600.mix(0.25))
        .label_style(&full_palette::GREY_600)
        .y_max_light_lines(0)
        .draw()
        .unwrap();
    (drawing_area, chart)
}

#[must_use]
fn energy_balance_chart(energy_profile: &energy::NewProfile) -> Markup {
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
        let values = points.iter().flat_map(|(_, balance)| *balance).map(|power| power.0);
        (
            values.clone().min_by(f64::total_cmp).unwrap_or_default(),
            values.max_by(f64::total_cmp).unwrap_or_default(),
        )
    };
    points.push((24.0, points[0].1));

    let mut svg = PreEscaped(String::new());
    {
        let (drawing_area, mut chart) = new_chart(&mut svg, min_y..max_y);
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.grid.import.0)),
                crate::web::plotters::DANGER.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.grid.export.0)),
                crate::web::plotters::LINK.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.battery.import.0)),
                crate::web::plotters::SUCCESS.stroke_width(2),
            ))
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                points.iter().map(|(x, balance)| (*x, balance.battery.export.0)),
                crate::web::plotters::WARNING.stroke_width(2),
            ))
            .unwrap();
        drawing_area.present().unwrap();
    }
    svg
}
