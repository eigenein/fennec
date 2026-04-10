#![cfg(feature = "cli")]

use anyhow::Error;
use clap::Parser;
use fennec_modbus::tcp::tokio::Client;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

type Result<T = (), E = Error> = core::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result {
    let env_filter =
        EnvFilter::builder().with_default_directive(LevelFilter::TRACE.into()).from_env()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time().compact().with_filter(env_filter))
        .init();

    let args = Args::parse();
    let client = Client::connect(args.endpoint).await?;
    let words = client.read_holding_registers(args.unit_id.try_into()?, args.address, 1).await?;
    println!("{}", words[0]);
    Ok(())
}

#[derive(Parser)]
struct Args {
    /// Connection endpoint.
    #[clap(long = "endpoint", env = "ENDPOINT")]
    endpoint: String,

    /// Unit ID aka «slave ID».
    #[clap(long = "unit-id", alias = "slave-id", env = "UNIT_ID")]
    unit_id: u8,

    /// Starting register address.
    #[clap(long = "address", env = "ADDRESS")]
    address: u16,
}
