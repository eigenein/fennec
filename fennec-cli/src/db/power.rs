use bon::Builder;
use chrono::{DateTime, Utc};
use mongodb::options::TimeseriesGranularity;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    db,
    quantity::{Zero, power::Watts, ratios::Percentage},
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
    #[serde(default, rename = "batteryV2")]
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
    #[serde(rename = "socPercent")]
    pub charge: Percentage,

    #[serde(rename = "externalWatts")]
    pub external: Watts,

    #[serde(rename = "internalWatts")]
    pub internal: Watts,

    /// Active EPS power.
    ///
    /// We track it separately, because in all modes, the battery serves the demand on this output
    /// and competes for the inverter maximum power.
    pub eps: Watts,
}

impl BatteryMeasurement {
    pub fn power_mode(self) -> Option<BatteryPowerMode> {
        if self.external == Watts::ZERO && self.internal <= Watts::ZERO {
            Some(BatteryPowerMode::Idle(-self.internal))
        } else if self.internal > Watts::ZERO && self.external < Watts::ZERO {
            Some(BatteryPowerMode::Charging(self.internal / -self.external))
        } else if self.internal < Watts::ZERO && self.external > Watts::ZERO {
            Some(BatteryPowerMode::Discharging(self.external / -self.internal))
        } else {
            None
        }
    }
}

/// Actual battery working mode based on the power measurements – unrelated to scheduled working mode.
#[derive(Copy, Clone)]
pub enum BatteryPowerMode {
    /// Idling with non-negative parasitic load.
    Idle(Watts),

    /// Charging mode with respective efficiency, `0.0..=1.0`.
    Charging(f64),

    /// Discharging mode with respective efficiency, `0.0..=1.0`.
    Discharging(f64),
}
