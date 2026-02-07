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

    /// No idea what this is. Observed behaviour (incomplete):
    ///
    /// - Charging: with excess PV power.
    #[serde(rename = "EasyMode")]
    EasyMode,

    /// Old inactive groups seem to be switched into this status.
    UnexpectedValue,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfUse => write!(f, "Self-use"),
            Self::FeedIn => write!(f, "Feed-in"),
            Self::ForceCharge => write!(f, "Force charge"),
            Self::ForceDischarge => write!(f, "Force discharge"),
            Self::Backup => write!(f, "Backup"),
            Self::EasyMode => write!(f, "Easy mode"),
            Self::UnexpectedValue => write!(f, "Unexpected value"),
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
            Self::EasyMode | Self::UnexpectedValue => Color::Reset,
        }
    }
}
