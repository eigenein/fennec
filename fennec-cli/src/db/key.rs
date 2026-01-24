#[derive(Copy, Clone, Debug)]
pub enum Key {
    SchemaVersion,

    #[cfg(test)]
    Test,
}

impl Key {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersion => "schema_version",

            #[cfg(test)]
            Self::Test => "test",
        }
    }
}
