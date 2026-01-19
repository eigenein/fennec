use anyhow::Context;
use serde::{Deserialize, de::DeserializeOwned};
use worker::Fetcher;

use crate::result::Result;

pub struct Client(Fetcher);

impl Client {
    /// Fetch the latest measurement.
    ///
    /// API docs: <https://api-documentation.homewizard.com/docs/v1/measurement>.
    pub async fn get_measurement<R: DeserializeOwned>(&self) -> Result<R> {
        self.0
            .fetch("http://host/api/v1/data", None)
            .await
            .context("failed to fetch the URL")?
            .json()
            .await
            .context("failed to deserialize the response")
    }
}

#[derive(Deserialize)]
pub struct EnergySocketMeasurement {
    #[serde(rename = "total_power_import_kwh")]
    pub total_power_import: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_socket_measurement_ok() -> Result {
        // language=json
        let body = r#"{
            "wifi_ssid": "SSID",
            "wifi_strength": 70,
            "total_power_import_kwh": 798.828,
            "total_power_import_t1_kwh": 798.828,
            "total_power_export_kwh": 649.451,
            "total_power_export_t1_kwh": 649.451,
            "active_power_w": 0.0,
            "active_power_l1_w": 0.0,
            "active_voltage_v": 236.335,
            "active_current_a": 0.386,
            "active_reactive_power_var": 0.0,
            "active_apparent_power_va": 0.0,
            "active_power_factor": 1.0,
            "active_frequency_hz": 49.99
        }"#;
        serde_json::from_str::<EnergySocketMeasurement>(body)?;
        Ok(())
    }
}
