#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Forced charging from any source.
    Charging,

    /// Forced discharging, no matter the actual consumption.
    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    Balancing,
}
