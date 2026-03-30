pub mod state;
mod status;

use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use axum::{Router, response::IntoResponse, routing::get};
use clap::crate_version;
use http::header;
use maud::{DOCTYPE, Markup, html};

use crate::prelude::*;

pub async fn serve(
    address: IpAddr,
    port: u16,
    state: Arc<Mutex<state::ApplicationState>>,
) -> Result {
    info!(%address, port, "serving web UI…");
    let app =
        Router::new().route("/favicon.svg", get(favicon)).route("/", get(index)).with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}

async fn favicon() -> impl IntoResponse {
    let icon = html! {
        svg xmlns="http://www.w3.org/2000/svg" "viewBox='0 0 100 100'" {
            text y="0.95em" font-size="90" { "🦊" }
        }
    };
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "Cache-Control: public, max-age=3600"),
        ],
        icon.into_string(),
    )
}

#[instrument(skip_all)]
async fn index(
    axum::extract::State(state): axum::extract::State<Arc<Mutex<state::ApplicationState>>>,
) -> Markup {
    let state = state.lock().unwrap();
    let markup = html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Fennec" }
                link rel="icon" href="/favicon.svg" type="image/svg+xml";
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/bulma/1.0.4/css/bulma.min.css" integrity="sha512-yh2RE0wZCVZeysGiqTwDTO/dKelCbS9bP2L94UvOFtl/FKXcNAje3Y2oBg/ZMZ3LS1sicYk4dYVGtDex75fvvA==" crossorigin="anonymous" referrerpolicy="no-referrer";
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/7.0.1/css/all.min.css" integrity="sha512-2SwdPD6INVrV/lHTZbO2nodKhrnDdJK9/kg2XD1r9uGqPo1cUbujc+IYdlYdEErWNu69gVcYgdxlmVmzTWnetw==" crossorigin="anonymous" referrerpolicy="no-referrer";
            }
            body {
                nav.navbar.(state.status()) role="navigation" aria-label="main navigation" {
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
                                    p.is-size-7.is-uppercase.has-text-weight-medium { (state.logger.status()) }
                                }
                            }
                            div.navbar-item {
                                div {
                                    p.is-size-7 { "solver" }
                                    p.is-size-7.is-uppercase.has-text-weight-medium { (state.solver.status()) }
                                }
                            }
                        }
                    }
                }
                section.section {
                    div.container {
                    }
                }
            }
        }
    };
    drop(state);
    markup
}
