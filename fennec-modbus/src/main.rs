#![cfg(feature = "cli")]

use anyhow::Error;
use clap::{Parser, Subcommand};
use fennec_modbus::{
    client::AsyncClient,
    protocol::function::read_registers::Holding,
    tcp::tokio::Client,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

type Result<T = (), E = Error> = core::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result {
    let args = Args::parse();
    let env_filter =
        EnvFilter::builder().with_default_directive(LevelFilter::TRACE.into()).from_env()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time().compact().with_filter(env_filter))
        .init();

    match args.command {
        Command::ReadHolding { n_values } => {
            let client = Client::builder().endpoint(args.endpoint).build();
            let unit_id = args.unit_id.try_into()?;
            let n_values = usize::from(n_values);
            let values: Vec<u16> =
                client.read_registers::<Holding, u16>(unit_id, args.address, n_values).await?;
            for value in values {
                println!("{value}");
            }
        }
    }

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

    #[clap(subcommand)]
    command: Command,
}

#[derive(Copy, Clone, Subcommand)]
enum Command {
    /// Read holding registers.
    ReadHolding {
        /// Number of values to read.
        #[clap(
        long = "n-values",
        env = "N_VALUES",
        default_value = "1",
        value_parser = clap::value_parser!(u8).range(1..)
    )]
        n_values: u8,
    },
}
