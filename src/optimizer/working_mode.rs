#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum WorkingMode {
    Charging,

    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    SelfUse,
}

/// Sequence of 1-hour long time slots with their respective working modes.
#[derive(derive_more::Deref, derive_more::AsRef, derive_more::From, derive_more::IntoIterator)]
pub struct WorkingModeSequence(Vec<WorkingMode>);
