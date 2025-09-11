use serde::Deserialize;

use crate::units::KilowattHours;

#[derive(Deserialize)]
pub struct RealTimeRawVariable {
    #[serde(rename = "variable")]
    pub name: String,

    pub value: serde_json::Value,

    pub unit: Option<String>,

    #[serde(rename = "name")]
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct DeviceRealTimeData {
    #[serde(rename = "deviceSN")]
    pub serial_number: String,

    #[serde(rename = "datas")]
    pub variables: Vec<RealTimeRawVariable>,
}

#[derive(Deserialize)]
pub struct DeviceVariables {
    #[serde(rename = "ResidualEnergy")]
    pub residual_energy: KilowattHours,
}

#[derive(Deserialize)]
pub struct DeviceDetails {
    #[serde(rename = "batteryList")]
    pub batteries: Vec<BatteryDetails>,
}

impl DeviceDetails {
    pub fn total_capacity(&self) -> KilowattHours {
        self.batteries
            .iter()
            .filter_map(|battery| {
                battery
                    .capacity_watt_hours
                    .map(|watt_hours| KilowattHours::from(watt_hours / 1000.0))
            })
            .sum()
    }
}

#[derive(Deserialize)]
pub struct BatteryDetails {
    #[serde(rename = "capacity")]
    pub capacity_watt_hours: Option<f64>,
}
