#[derive(Copy, Clone, Debug)]
pub enum Key {
    /// Database schema version – used for migrations.
    SchemaVersion,

    /// Last known battery residual energy in milliwatt-hours – used to track its transitions.
    BatteryResidualEnergy,

    #[cfg(test)]
    Test,
}

impl Key {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersion => "schema_version",
            Self::BatteryResidualEnergy => "battery::last_known_residual_millis",

            #[cfg(test)]
            Self::Test => "test",
        }
    }
}
