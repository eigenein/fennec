use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum WorkingMode {
    /// Do not do anything.
    #[default]
    #[serde(rename = "R", alias = "M")]
    Retaining,

    /// Forced charging on any power.
    #[serde(rename = "C")]
    Charging,

    /// Forced discharging, no matter the actual consumption.
    #[serde(rename = "D")]
    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    #[serde(rename = "B")]
    Balancing,
}
