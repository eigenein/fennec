mod battery;
mod handlers;
mod partials;
mod plotters;
mod working_mode;

use std::{net::IpAddr, sync::Arc};

use axum::{Router, routing::get};
use tokio::sync::RwLock;

use crate::{
    cli::{hunter, logger},
    prelude::*,
};

#[derive(Clone)]
pub struct State {
    pub hunter: Arc<RwLock<hunter::State>>,
    pub logger_runner: logger::Runner,
}

pub async fn serve(address: IpAddr, port: u16, state: State) -> Result {
    info!(%address, port, "serving web UI…");
    let app = Router::new()
        .route("/", get(handlers::index::get))
        .route(handlers::energy_profile::PATH, get(handlers::energy_profile::get))
        .route("/readiness", get(handlers::readiness::get))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((address, port)).await?;
    axum::serve(listener, app).await.context("the web application has failed")
}
