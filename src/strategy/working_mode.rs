#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, prost::Enumeration)]
pub enum WorkingMode {
    /// Do not do anything.
    Maintaining = 0,

    /// Forced charging on any power.
    Charging = 1,

    /// Forced discharging, no matter the actual consumption.
    Discharging = 2,

    /// Charge on excess PV power, discharge on insufficient PV power.
    Balancing = 3,
}
