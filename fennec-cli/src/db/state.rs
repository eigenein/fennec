use bson::doc;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::quantity::{energy::MilliwattHours, power::Kilowatts};

pub trait State: Serialize + DeserializeOwned {
    const ID: &str;
}

/// Last known battery residual energy.
#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize, From, Into)]
pub struct BatteryResidualEnergy {
    #[serde(rename = "milliwattHours")]
    residual_energy: MilliwattHours,
}

impl State for BatteryResidualEnergy {
    const ID: &str = "batteryResidualEnergy";
}

#[must_use]
#[derive(Copy, Clone, Default, Serialize, Deserialize, From, Into)]
pub struct HourlyStandByPower {
    #[serde(rename = "kilowatts")]
    hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl State for HourlyStandByPower {
    const ID: &str = "hourlyStandByPower";
}
