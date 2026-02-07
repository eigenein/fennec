use std::fmt::{Display, Formatter};

use comfy_table::Color;
use serde::{Deserialize, Serialize};

/// FoxESS cloud working modes.
///
/// The descriptions in the app do match the actual function on my MQ2200.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingMode {
    /// «Self-use» per the app. Observed behaviour:
    ///
    /// - Charging: with excess PV power.
    /// - Discharging: compensating the PV power deficit.
    #[serde(rename = "SelfUse")]
    SelfUse,

    /// «Load priority» per the app. Observed behaviour:
    ///
    /// - Charging: never, excess PV power is exported.
    /// - Discharging: compensating the PV power deficit.
    #[serde(rename = "Feedin")]
    FeedIn,

    /// «Battery priority» per the app. Observed behaviour:
    ///
    /// - Charging: with excess PV power.
    /// - Discharging: never.
    #[serde(rename = "Backup")]
    Backup,

    #[serde(rename = "ForceCharge")]
    ForceCharge,

    #[serde(rename = "ForceDischarge")]
    ForceDischarge,

    /// I have no idea what this is. Observed behaviour (incomplete):
    ///
    /// - Charging: with excess PV power.
    #[serde(rename = "EasyMode")]
    EasyMode,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfUse => write!(f, "Self-use"),
            Self::FeedIn => write!(f, "Load priority"),
            Self::ForceCharge => write!(f, "Forced charge"),
            Self::ForceDischarge => write!(f, "Forced discharge"),
            Self::Backup => write!(f, "Battery priority"),
            Self::EasyMode => write!(f, "Easy mode"),
        }
    }
}

impl WorkingMode {
    pub const fn color(self) -> Color {
        match self {
            Self::ForceDischarge => Color::Blue,
            Self::ForceCharge => Color::Green,
            Self::SelfUse => Color::DarkYellow,
            Self::FeedIn => Color::Magenta,
            Self::Backup => Color::Cyan,
            Self::EasyMode => Color::Reset,
        }
    }
}
