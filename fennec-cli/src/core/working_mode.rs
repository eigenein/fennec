use comfy_table::{Attribute, Color};

#[derive(Debug, clap::ValueEnum, enumset::EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Only excess solar power charging without discharging.
    Backup,

    /// Charge on excess solar power, discharge on insufficient solar power.
    Balance,

    /// Forced charging from any source.
    Charge,

    /// Forced discharging, no matter the actual consumption.
    Discharge,
}

impl WorkingMode {
    pub const fn color(self) -> Color {
        match self {
            Self::Charge => Color::Green,
            Self::Discharge => Color::Blue,
            Self::Balance => Color::DarkYellow,
            Self::Backup => Color::Cyan,
            Self::Idle => Color::Reset,
        }
    }

    pub const fn attribute(self) -> Attribute {
        match self {
            Self::Balance | Self::Charge | Self::Discharge => Attribute::Bold,
            Self::Idle | Self::Backup => Attribute::NoBold,
        }
    }
}
