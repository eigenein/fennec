mod color;
pub mod state;

use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use axum::{Router, routing::get};
use chrono_humanize::HumanTime;
use maud::{DOCTYPE, Markup, html};

use crate::{prelude::*, web::color::Color};

pub async fn serve(address: IpAddr, port: u16, state: Arc<Mutex<state::Application>>) -> Result {
    info!(%address, port, "serving web UI…");
    let app = Router::new().route("/", get(index)).with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}

#[instrument(skip_all)]
async fn index(
    axum::extract::State(state): axum::extract::State<Arc<Mutex<state::Application>>>,
) -> Markup {
    #[expect(clippy::significant_drop_tightening)]
    let state = state.lock().unwrap();

    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Fennec" }
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@1.0.4/css/bulma.min.css";
                link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='0.95em' font-size='90'>🦊</text></svg>";
            }
            body {
                // TODO: pick `is-success` or `is-danger` based on the status.
                section.hero.is-small.(Color::Success) {
                    div.hero-head {
                        nav.navbar.(Color::Success) role="navigation" aria-label="main navigation" {
                            div.container {
                                div."navbar-brand" {
                                    a."navbar-item" href="/" {
                                        svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100" {
                                            text y="0.95em" font-size="90" { "🦊" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div.hero-body {
                        nav.level {
                            div.level-item.has-text-centered {
                                div {
                                    p.heading { "Logger" }
                                    @match state.logger {
                                        Some(Ok(last_log_timestamp)) => {
                                            p.title title=(last_log_timestamp) { (HumanTime::from(last_log_timestamp)) }
                                        }
                                        Some(Err(_)) => p.title { "failed" },
                                        None => p.title { "pending" },
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
