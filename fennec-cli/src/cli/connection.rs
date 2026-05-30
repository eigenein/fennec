use std::sync::Arc;

use clap::Parser;

use crate::{
    api::{battery, homewizard},
    prelude::*,
};

#[derive(Parser)]
pub struct ConnectionArgs {
    /// P1 meter measurement URL.
    #[clap(long, env = "GRID_MEASUREMENT_URL")]
    grid_measurement_url: homewizard::Url,

    /// Battery Modbus address. Currently, only FoxESS MQ2200 is supported.
    #[clap(long = "battery-address", env = "BATTERY_ADDRESS")]
    battery_address: String,
}

impl ConnectionArgs {
    pub fn connect(self) -> Result<Connections> {
        Ok(Connections {
            grid_measurement: self.grid_measurement_url.client()?,
            battery: Arc::new(battery::Client::new(self.battery_address)),
        })
    }
}

#[derive(Clone)]
pub struct Connections {
    pub grid_measurement: homewizard::Client,
    pub battery: Arc<battery::Client>,
}
