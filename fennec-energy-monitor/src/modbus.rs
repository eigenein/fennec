use anyhow::Context;
use fennec_quantities::energy::KilowattHours;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use worker::Fetcher;

use crate::result::Result;

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct BatteryStatus {
    pub state_of_charge: f64,
    pub state_of_health: f64,

    #[serde(rename = "design_capacity_kwh")]
    pub design_capacity: KilowattHours,
}

pub struct Client(pub Fetcher);

impl Client {
    #[instrument(skip_all)]
    pub async fn get_battery_status(&self) -> Result<BatteryStatus> {
        info!("fetching the battery status…");
        self.0
            .fetch("http://modbus-proxy/battery-status", None)
            .await
            .context("failed to fetch the URL")?
            .json()
            .await
            .context("failed to deserialize the response")
    }
}
