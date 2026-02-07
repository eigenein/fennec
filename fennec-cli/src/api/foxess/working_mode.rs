use std::fmt::{Display, Formatter};

use comfy_table::Color;
use serde::{Deserialize, Serialize};

/// Working modes per FoxESS API and their respective titles per the Fox Cloud app.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingMode {
    #[serde(rename = "SelfUse")]
    SelfUse,

    #[serde(rename = "Feedin")]
    LoadPriority,

    #[serde(rename = "ForceCharge")]
    ForcedCharge,

    #[serde(rename = "ForceDischarge")]
    ForcedDischarge,

    #[serde(rename = "Backup")]
    BatteryPriority,

    #[serde(rename = "EasyMode")]
    EasyMode,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfUse => write!(f, "Self-use"),
            Self::LoadPriority => write!(f, "Load priority"),
            Self::ForcedCharge => write!(f, "Forced charge"),
            Self::ForcedDischarge => write!(f, "Forced discharge"),
            Self::BatteryPriority => write!(f, "Battery priority"),
            Self::EasyMode => write!(f, "Easy mode"),
        }
    }
}

impl WorkingMode {
    pub const fn color(self) -> Color {
        match self {
            Self::ForcedDischarge => Color::Blue,
            Self::ForcedCharge => Color::Green,
            Self::SelfUse => Color::DarkYellow,
            Self::LoadPriority => Color::Magenta,
            Self::BatteryPriority => Color::Cyan,
            Self::EasyMode => Color::Reset,
        }
    }
}
