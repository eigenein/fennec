#[derive(Copy, Clone, Debug)]
pub enum LegacyKey {
    /// Last known battery residual energy in milliwatt-hours – used to track its transitions.
    BatteryResidualEnergy,

    #[cfg(test)]
    Test,
}

impl LegacyKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BatteryResidualEnergy => "battery::last_known_residual_millis",

            #[cfg(test)]
            Self::Test => "test",
        }
    }
}
