pub mod application;
mod battery;
mod handlers;
mod partials;
mod plotters;
mod working_mode;

use std::net::IpAddr;

use axum::{Router, routing::get};

use crate::prelude::*;

pub async fn serve(address: IpAddr, port: u16, state: application::State) -> Result {
    info!(%address, port, "serving web UI…");
    let app = Router::new()
        .route("/", get(handlers::index::get))
        .route(handlers::energy_balance::PATH, get(handlers::energy_balance::get))
        .route("/readiness", get(handlers::readiness::get))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}
