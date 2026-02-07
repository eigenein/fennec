use comfy_table::Color;

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
}
