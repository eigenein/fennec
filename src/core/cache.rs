use std::{fmt::Debug, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    core::{series::Series, working_mode::WorkingMode},
    prelude::*,
    units::energy::KilowattHours,
};

#[derive(Default, Serialize, Deserialize)]
pub struct Cache {
    #[serde(default, rename = "solution")]
    pub solution: Series<WorkingMode>,

    #[serde(default)]
    pub total_usage: Series<KilowattHours>,
}

impl Cache {
    #[instrument(name = "Reading the cache…")]
    pub fn read_from<P: AsRef<Path> + Debug>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if path.is_file() { Ok(toml::from_slice(&fs::read(path)?)?) } else { Ok(Self::default()) }
    }

    #[instrument(skip(self), name = "Writing the cache…")]
    pub fn write_to<P: AsRef<Path> + Debug>(&self, path: P) -> Result {
        fs::write(path, toml::to_string(self)?)?;
        Ok(())
    }
}
