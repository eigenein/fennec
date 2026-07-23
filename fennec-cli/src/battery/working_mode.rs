use std::fmt::{Display, Formatter};

/// Ordered by priority: least battery action first.
/// It matters when the corresponding solution losses are similar.
#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, clap::ValueEnum)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Only excess solar power charging without discharging.
    Harness,

    /// Compensate on insufficient solar power, but never charge.
    Compensate,

    /// Charge on excess solar power, compensate on insufficient solar power.
    SelfUse,

    /// Forced charging from any source.
    Charge,

    /// Forced discharging, no matter the actual consumption.
    Discharge,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::SelfUse => "Self-use",
            Self::Idle => "Idle",
            Self::Harness => "Harness",
            Self::Charge => "Charge",
            Self::Discharge => "Discharge",
            Self::Compensate => "Compensate",
        };
        text.fmt(f)
    }
}
