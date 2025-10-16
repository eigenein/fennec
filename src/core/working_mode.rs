#[derive(Debug, clap::ValueEnum, enumset::EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Charge on excess solar power, discharge on insufficient solar power.
    Balancing,

    /// Forced charging from any source.
    Charging,

    /// Forced discharging, no matter the actual consumption.
    Discharging,
}
