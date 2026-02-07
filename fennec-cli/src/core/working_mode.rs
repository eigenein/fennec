use std::fmt::{Display, Formatter};

use comfy_table::Color;

#[derive(Debug, clap::ValueEnum, enumset::EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Only excess solar power charging without discharging.
    Harvest,

    /// Charge on excess solar power, compensate on insufficient solar power.
    SelfUse,

    /// Forced charging from any source.
    Charge,

    /// Forced discharging, no matter the actual consumption.
    Discharge,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfUse => write!(f, "Self-use"),
            Self::Idle => write!(f, "Idle"),
            Self::Harvest => write!(f, "Harvest"),
            Self::Charge => write!(f, "Charge"),
            Self::Discharge => write!(f, "Discharge"),
        }
    }
}

impl WorkingMode {
    pub const fn color(self) -> Color {
        match self {
            Self::Charge => Color::Green,
            Self::Discharge => Color::Blue,
            Self::SelfUse => Color::DarkYellow,
            Self::Harvest => Color::Cyan,
            Self::Idle => Color::Reset,
        }
    }
}
