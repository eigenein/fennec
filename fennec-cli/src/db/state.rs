use bson::doc;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::quantity::energy::MilliwattHours;

pub trait ApplicationState: Serialize + DeserializeOwned {
    const ID: &str;
}

/// Last known battery residual energy.
#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize, From, Into)]
pub struct BatteryResidualEnergy {
    #[serde(rename = "milliwattHours")]
    residual_energy: MilliwattHours,
}

impl ApplicationState for BatteryResidualEnergy {
    const ID: &str = "batteryResidualEnergy";
}
