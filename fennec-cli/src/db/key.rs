#[derive(Copy, Clone, Debug)]
pub enum Key {
    SchemaVersion,
}

impl Key {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersion => "schema_version",
        }
    }
}
