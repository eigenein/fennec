use std::sync::Arc;

use clap::Parser;
use reqwest::Url;
use tokio::sync::Mutex;

use crate::{
    api::{fox_cloud, homewizard, modbus::foxess::MQ2200},
    cli::fox_cloud::FoxCloudConnectionArgs,
    db::Db,
    prelude::*,
};

#[derive(Parser)]
pub struct ConnectionArgs {
    /// P1 meter measurement URL.
    #[clap(long, env = "GRID_MEASUREMENT_URL")]
    grid_measurement_url: homewizard::Url,

    #[clap(long = "mongodb-uri", env = "MONGODB_URI")]
    db_uri: Url,

    /// Battery Modbus address. Currently, only FoxESS MQ2200 is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    battery_address: String,

    #[clap(flatten)]
    fox_cloud: FoxCloudConnectionArgs,
}

impl ConnectionArgs {
    pub async fn connect(self) -> Result<Connections> {
        Ok(Connections {
            grid_measurement: self.grid_measurement_url.client()?,
            db: Db::with_uri(self.db_uri).await?,
            battery: Arc::new(Mutex::new(MQ2200::connect(self.battery_address).await?)),
            fox_cloud: self.fox_cloud.client()?,
        })
    }
}

#[derive(Clone)]
pub struct Connections {
    pub grid_measurement: homewizard::Client,
    pub db: Db,
    pub battery: Arc<Mutex<MQ2200>>,
    pub fox_cloud: Option<fox_cloud::Client>,
}
