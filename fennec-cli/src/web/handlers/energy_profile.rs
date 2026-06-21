use axum::extract::State;
use chrono::{Local, NaiveTime, TimeDelta, TimeZone};
use itertools::Itertools;
use maud::{Markup, PreEscaped, html};
use plotters::{backend::SVGBackend, chart::ChartBuilder, prelude::*};

use crate::{
    energy,
    energy::Balance,
    ops::chrono::Interval,
    prelude::*,
    quantity::power::Watts,
    web,
    web::partials,
};

pub const PATH: &str = "/energy-profile";

#[instrument(skip_all)]
#[expect(clippy::too_many_lines)]
#[expect(clippy::significant_drop_tightening)]
pub async fn get(State(state): State<web::State>) -> Markup {
    debug!("access");

    let energy_profile = state.logger.read().await;
    let mean_balance = energy_profile.mean_balance.0;

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

            section.section.py-5 {
                div.card {
                    header.card-header {
                        p.card-header-title { "Instant balance" }
                    }
                    div.card-content {
                        figure.image.has-plotters-fix {
                            (instant_balance_chart(&energy_profile))
                        }
                    }
                }
            }

            section.section.py-5 {
                div.card {
                    header.card-header {
                        p.card-header-title { "Interval balance" }
                    }
                    div.card-content {
                        figure.image.has-plotters-fix {
                            (interval_balance_chart(&energy_profile))
                        }
                    }
                }
            }

            section.section.py-5 {
                div.card {
                    header.card-header {
                        p.card-header-title { "Harmonics" }
                    }
                    div.card-content {
                        div.table-container {
                            table.table.is-striped.is-narrow.is-hoverable.is-fullwidth {
                                thead {
                                    tr {
                                        th.has-text-centered rowspan="2" { "Fourier" (PreEscaped("<br>")) "mode" }
                                        th.has-text-success align="center" colspan="2" { "Battery import" }
                                        th.has-text-warning align="center" colspan="2" { "Battery export" }
                                        th.has-text-danger align="center" colspan="2" { "Grid import" }
                                        th.has-text-link align="center" colspan="2" { "Grid export" }
                                    }
                                    tr {
                                        th.has-text-right.has-text-success { "Cosine" }
                                        th.has-text-right.has-text-success { "Sine" }
                                        th.has-text-right.has-text-warning { "Cosine" }
                                        th.has-text-right.has-text-warning { "Sine" }
                                        th.has-text-right.has-text-danger { "Cosine" }
                                        th.has-text-right.has-text-danger { "Sine" }
                                        th.has-text-right.has-text-link { "Cosine" }
                                        th.has-text-right.has-text-link { "Sine" }
                                    }
                                }
                                tbody {
                                    @for (mode_index, harmonic) in (1..).zip(&energy_profile.balance_harmonics) {
                                        tr {
                                            th.has-text-right { "#" (mode_index) }
                                            td.has-text-right.has-text-success { (harmonic.0.cosine.battery.import) }
                                            td.has-text-right.has-text-success { (harmonic.0.sine.battery.import) }
                                            td.has-text-right.has-text-warning { (harmonic.0.cosine.battery.export) }
                                            td.has-text-right.has-text-warning { (harmonic.0.sine.battery.export) }
                                            td.has-text-right.has-text-danger { (harmonic.0.cosine.grid.import) }
                                            td.has-text-right.has-text-danger { (harmonic.0.sine.grid.import) }
                                            td.has-text-right.has-text-link { (harmonic.0.cosine.grid.export) }
                                            td.has-text-right.has-text-link { (harmonic.0.sine.grid.export) }
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

#[must_use]
fn render_chart(points: &[(f64, Balance<Watts>)]) -> Markup {
    let value_range = {
        let values = points.iter().flat_map(|(_, balance)| *balance).map(|power| power.0);
        values.clone().min_by(f64::total_cmp).unwrap_or_default()
            ..values.max_by(f64::total_cmp).unwrap_or_default()
    };

    let mut buf = PreEscaped(String::new());
    {
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
    buf
}

#[must_use]
fn instant_balance_chart(energy_profile: &energy::Profile) -> Markup {
    let mut points = {
        let mean_balance = energy_profile.mean_balance.0;
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
    points.push((24.0, points[0].1));
    render_chart(&points)
}

#[must_use]
fn interval_balance_chart(energy_profile: &energy::Profile) -> Markup {
    let points = {
        (0..24)
            .map(|hour| {
                let start = Local.with_ymd_and_hms(2026, 1, 1, hour, 0, 0).unwrap();
                let interval = Interval::new(start, start + TimeDelta::hours(1));
                (f64::from(hour) + 0.5, energy_profile.mean_balance_over(interval))
            })
            .collect_vec()
    };
    render_chart(&points)
}
