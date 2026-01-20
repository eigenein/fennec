use anyhow::{Context, bail};
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
        let mut response = self
            .0
            .fetch("http://fennec-modbus-proxy/battery-status", None)
            .await
            .context("failed to fetch the Modbus proxy URL")?;
        if response.status_code() != 200 {
            bail!("Modbus proxy returned {}", response.status_code());
        }
        response.json().await.context("failed to deserialize the response")
    }
}
