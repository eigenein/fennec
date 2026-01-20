mod cli;
mod result;

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use clap::{Parser, crate_version};
use serde::Serialize;
use tokio::net::{TcpListener, lookup_host};
use tokio_modbus::{
    Slave,
    client::{Client, Reader},
};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info, instrument};

use crate::{
    cli::{Args, BatteryArgs},
    result::Result,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().without_time().compact().init();
    let args = Args::parse();
    info!(version = crate_version!(), args.bind_address, "Starting…");

    let listener =
        TcpListener::bind(args.bind_address).await.context("failed to bind to the address")?;
    let state = AppState { battery_args: args.battery };
    let app = Router::new()
        .route("/battery-status", get(get_battery_status))
        .with_state(Arc::new(state))
        .layer((TraceLayer::new_for_http(), TimeoutLayer::new(Duration::from_secs(10))));

    info!("Serving…");
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    Ok(())
}

/// Per <https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs>.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

struct AppState {
    battery_args: BatteryArgs,
}

#[derive(Serialize)]
struct BatteryStatus {
    state_of_charge: f64,
    state_of_health: f64,
    design_capacity_kwh: f64,
}

#[instrument(skip_all)]
async fn get_battery_status(
    State(args): State<Arc<AppState>>,
) -> Result<Json<BatteryStatus>, StatusCode> {
    match get_battery_status_internal(&args.battery_args).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            error!("Failed to retrieve the battery status: {error:#}");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

async fn get_battery_status_internal(args: &BatteryArgs) -> Result<BatteryStatus> {
    let address = lookup_host(&args.connection.address)
        .await
        .context("failed to resolve the battery address")?
        .next()
        .context("no addresses resolved for battery address")?;
    info!(%address, "Resolved the battery address");
    let mut context =
        tokio_modbus::client::tcp::connect_slave(address, Slave(args.connection.slave_id))
            .await
            .context("failed to connect to the battery")?;
    info!("Connected via Modbus");
    let response = {
        let state_of_charge_percentage = *context
            .read_holding_registers(args.registers.state_of_charge, 1)
            .await??
            .first()
            .context("empty SoC register response")?;
        let state_of_health_percentage = *context
            .read_holding_registers(args.registers.state_of_health, 1)
            .await??
            .first()
            .context("empty SoH register response")?;
        let design_energy_decawatts = *context
            .read_holding_registers(args.registers.design_energy, 1)
            .await??
            .first()
            .context("empty SoH register response")?;
        BatteryStatus {
            state_of_charge: f64::from(state_of_charge_percentage) / 100.0,
            state_of_health: f64::from(state_of_health_percentage) / 100.0,
            design_capacity_kwh: f64::from(design_energy_decawatts) * 0.01,
        }
    };
    info!(response.state_of_charge, response.state_of_health, response.design_capacity_kwh);
    let _ = context.disconnect().await;
    Ok(response)
}
