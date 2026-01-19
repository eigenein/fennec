use anyhow::Context;
use quantities::energy::KilowattHours;
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
pub struct PowerMeasurement {
    #[serde(rename = "total_power_import_kwh")]
    pub total_power_import: KilowattHours,

    #[serde(rename = "total_power_export_kwh")]
    pub total_power_export: KilowattHours,
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
        serde_json::from_str::<PowerMeasurement>(body)?;
        Ok(())
    }

    #[test]
    fn p1_measurement_ok() -> Result {
        // language=json
        let body = r#"{
            "wifi_ssid": "SSID",
            "wifi_strength": 64,
            "smr_version": 50,
            "meter_model": "ISKRA 2M550E-1012",
            "unique_id": "...",
            "active_tariff": 2,
            "total_power_import_kwh": 35264.809,
            "total_power_import_t1_kwh": 18070.244,
            "total_power_import_t2_kwh": 17194.565,
            "total_power_export_kwh": 7867.813,
            "total_power_export_t1_kwh": 2425.682,
            "total_power_export_t2_kwh": 5442.131,
            "active_power_w": -11.0,
            "active_power_l1_w": -19.0,
            "active_voltage_l1_v": 235.1,
            "active_current_a": 0.081,
            "active_current_l1_a": -0.081,
            "voltage_sag_l1_count": 13.0,
            "voltage_swell_l1_count": 10.0,
            "any_power_fail_count": 9.0,
            "long_power_fail_count": 7.0,
            "total_gas_m3": 10326.681,
            "gas_timestamp": 260119145509,
            "gas_unique_id": "...",
            "external": [
                {
                    "unique_id": "...",
                    "type": "gas_meter",
                    "timestamp": 260119145509,
                    "value": 10326.681,
                    "unit": "m3"
                }
            ]
        }"#;
        serde_json::from_str::<PowerMeasurement>(body)?;
        Ok(())
    }
}
