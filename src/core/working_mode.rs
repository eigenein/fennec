use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum WorkingMode {
    /// Do not do anything.
    #[serde(alias = "I", alias = "M", alias = "R")]
    Idle,

    /// Forced charging on any power.
    #[serde(alias = "C")]
    Charging,

    /// Forced discharging, no matter the actual consumption.
    #[serde(alias = "D")]
    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    #[default]
    #[serde(alias = "B")]
    Balancing,
}
