use bon::Builder;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    db,
    quantity::{Zero, energy::WattHours, power::Watts},
};

/// Net power balance measurement.
#[serde_as]
#[derive(derive_more::Debug, Serialize, Deserialize, Builder)]
pub struct Measurement {
    #[serde_as(as = "bson::serde_helpers::datetime::FromChrono04DateTime")]
    #[serde(rename = "timestamp")]
    #[builder(default = Utc::now())]
    #[debug(skip)]
    pub timestamp: DateTime<Utc>,

    /// Net power deficit on the main connection.
    ///
    /// Positive is net consumption, negative is net production.
    ///
    /// This is equal to «P1 net consumption plus battery net production»,
    /// as we only need to track and compensate the net deficit, hence:
    ///
    /// - battery charging or discharging has no effect on it;
    /// - PV production lowers it.
    #[serde(rename = "netWatts")]
    pub net_deficit: Watts,

    /// TODO: make non-optional and rename.
    #[serde(default, rename = "batteryV4")]
    pub battery: Option<BatteryMeasurement>,

    /// TODO: remove in favour of the `battery` attribute.
    #[serde(rename = "epsActivePower", default = "default_eps_active_power")]
    pub eps_active_power: Watts,
}

impl db::Measurement for Measurement {
    const COLLECTION_NAME: &str = "powerMeasurements";
    const GRANULARITY: TimeseriesGranularity = TimeseriesGranularity::Seconds;
}

/// Fallback to read the old logs when the EPS load was missing completely.
///
/// TODO: it can be removed when the old measurements get deleted by the TTL.
const fn default_eps_active_power() -> Watts {
    Watts::ZERO
}

/// Battery power measurements.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Builder)]
pub struct BatteryMeasurement {
    #[serde(rename = "residualEnergyWattHours")]
    pub residual_energy: WattHours,

    #[serde(rename = "activePowerWatts")]
    pub active_power: Watts,

    /// EPS active power.
    ///
    /// We track it separately, because in all modes, the battery serves the demand on this output
    /// and competes for the inverter maximum power.
    #[serde(rename = "epsActivePowerWatts")]
    pub eps_active_power: Watts,
}
