use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{prelude::*, strategy::WorkingModeSchedule};

#[derive(Default, bon::Builder, Serialize, Deserialize)]
pub struct Cache {
    pub working_mode_schedule: WorkingModeSchedule,
}

impl Cache {
    #[instrument(skip_all, fields(path = %path.display()), name = "Reading cache…")]
    pub fn read_from(path: &Path) -> Result<Self> {
        if path.is_file() {
            serde_json::from_slice(&std::fs::read(path)?).context("failed to decode the cache file")
        } else {
            warn!("Cache file is not found");
            Ok(Self::default())
        }
    }

    #[instrument(skip_all, fields(path = %path.display()), name = "Writing cache…")]
    pub fn write_to(&self, path: &Path) -> Result {
        std::fs::write(path, serde_json::to_string(&self)?)
            .context("failed to write the cache file")
    }
}
