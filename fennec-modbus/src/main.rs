#![cfg(feature = "cli")]

use anyhow::Error;
use clap::{Parser, Subcommand};
use fennec_modbus::{
    protocol::function::{ReadRegisters, read_registers, read_registers::Holding},
    tcp::{UnitId, tokio::Client},
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
            let client = args.endpoint.client();
            let values: Vec<u16> = client
                .call::<ReadRegisters<Holding, Vec<u16>>>(
                    args.unit_id,
                    read_registers::Args::new(args.address, n_values.into())?,
                )
                .await?;
            for value in values {
                println!("{value}");
            }
        }
    }

    Ok(())
}

#[derive(Parser)]
struct Args {
    #[clap(flatten)]
    endpoint: Endpoint,

    /// Unit ID aka «slave ID».
    #[clap(long = "unit-id", alias = "slave-id", env = "UNIT_ID")]
    unit_id: UnitId,

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

#[derive(Parser)]
struct Endpoint {
    /// Connection endpoint.
    #[clap(long = "endpoint", env = "ENDPOINT")]
    inner: String,
}

impl Endpoint {
    pub fn client(self) -> Client<String> {
        Client::new(self.inner)
    }
}
