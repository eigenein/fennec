use std::{fmt::Debug, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{prelude::*, strategy::WorkingMode};

#[derive(Serialize, Deserialize)]
pub struct Cache {
    #[serde(default, rename = "schedule")]
    pub schedule: [WorkingMode; 24],
}

impl Cache {
    #[instrument(name = "Reading the cache…")]
    pub fn read_from<P: AsRef<Path> + Debug>(path: P) -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    #[instrument(skip(self), name = "Writing the cache…")]
    pub fn write_to<P: AsRef<Path> + Debug>(&self, path: P) -> Result {
        fs::write(path, serde_json::to_string(self)?)?;
        Ok(())
    }
}
