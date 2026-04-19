#![cfg(feature = "cli")]

use anyhow::Error;
use clap::{Parser, Subcommand};
use fennec_modbus::{
    contrib::mq2200,
    protocol::{address, address::Stride, function::read_multiple::AddressRange},
    tcp::{UnitId, tokio::Client},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

type Result<T = (), E = Error> = core::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result {
    let args = Args::parse();
    let env_filter =
        EnvFilter::builder().with_default_directive(LevelFilter::WARN.into()).from_env()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().without_time().compact().with_filter(env_filter))
        .init();

    let client = args.endpoint.client();
    let unit_id = args.unit_id;

    match args.command {
        Command::Test(device) => match device {
            Device::Mq2200 => {
                println!(
                    "State-of-health: {:?}",
                    client.call::<mq2200::ReadStateOfHealth>(unit_id, address::Const).await?.0
                );
                println!(
                    "Design capacity: {:?}",
                    client.call::<mq2200::ReadDesignCapacity>(unit_id, address::Const).await?.0
                );
                println!(
                    "Total active power: {:?}",
                    client.call::<mq2200::ReadTotalActivePower>(unit_id, address::Const).await?.0
                );
                println!(
                    "Total EPS active power: {:?}",
                    client.call::<mq2200::ReadEpsActivePower>(unit_id, address::Const).await?.0
                );
                println!(
                    "State-of-charge: {:?}",
                    client.call::<mq2200::ReadStateOfCharge>(unit_id, address::Const).await?.0
                );
                println!(
                    "Minimum system SoC: {:?}",
                    client
                        .call::<mq2200::ReadMinimumSystemStateOfCharge>(unit_id, address::Const)
                        .await?
                        .0
                );
                println!(
                    "Maximum SoC: {:?}",
                    client
                        .call::<mq2200::ReadMaximumStateOfCharge>(unit_id, address::Const)
                        .await?
                        .0
                );
                println!(
                    "Minimum SoC on grid: {:?}",
                    client
                        .call::<mq2200::ReadMinimumStateOfChargeOnGrid>(unit_id, address::Const)
                        .await?
                        .0
                );
                for i in 0..96 {
                    println!(
                        "Schedule #{i}: {:?}",
                        client
                            .call::<mq2200::ReadScheduleEntry>(
                                unit_id,
                                AddressRange::new(Stride::new(i))
                            )
                            .await?
                            .0
                    );
                }
            }
        },
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

    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
struct Endpoint {
    /// Connection endpoint.
    #[clap(name = "ENDPOINT", env = "ENDPOINT")]
    inner: String,
}

impl Endpoint {
    pub fn client(self) -> Client<String> {
        Client::new(self.inner)
    }
}

#[derive(Copy, Clone, Subcommand)]
enum Command {
    /// Test readings from a live device.
    #[clap(subcommand)]
    Test(Device),
}

#[derive(Copy, Clone, Subcommand)]
enum Device {
    /// Fox ESS MQ2200 (Mini Qube), Solakon ONE, and Avocado 22 Pro.
    #[clap(alias = "solakon-one", alias = "avocado-22-pro")]
    Mq2200,
}
