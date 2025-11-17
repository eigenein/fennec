#[derive(Debug, clap::ValueEnum, enumset::EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Only excess solar power charging without discharging.
    Backup,

    /// Charge on excess solar power, discharge on insufficient solar power.
    Balance,

    /// Forced charging from any source at 50% of the maximum charging power.
    ChargeSlowly,

    /// Forced charging from any source.
    Charge,

    /// Forced discharging, no matter the actual consumption.
    Discharge,
}
