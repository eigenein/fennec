#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum WorkingMode {
    /// Forced charging on any power.
    Charging,

    /// Forced discharging, no matter the actual consumption.
    Discharging,

    /// Charge on excess PV power, discharge on insufficient PV power.
    #[allow(dead_code)]
    Balancing,

    /// Do not do anything.
    #[default]
    Maintaining,
}
