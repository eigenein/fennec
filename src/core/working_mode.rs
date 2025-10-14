#[derive(Debug, clap::ValueEnum, enumset::EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Charge on excess PV power, discharge on insufficient PV power.
    Balancing,

    /// Forced charging from any source.
    Charging,

    /// Forced discharging, no matter the actual consumption.
    Discharging,
}
