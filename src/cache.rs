use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{prelude::*, strategy::WorkingMode};

#[derive(Default, bon::Builder, Serialize, Deserialize)]
pub struct Cache {
    pub working_mode_schedule: [WorkingMode; 24],
}

impl Cache {
    #[instrument(skip_all, fields(path = %path.display()), name = "Reading cache…")]
    pub fn read_from(path: &Path) -> Result<Self> {
        if path.is_file() {
            Ok(serde_json::from_slice(&std::fs::read(path)?)?)
        } else {
            warn!("Cache file is not found");
            Ok(Self::default())
        }
    }

    #[instrument(skip_all, fields(path = %path.display()), name = "Writing cache…")]
    pub fn write_to(&self, path: &Path) -> Result {
        Ok(std::fs::write(path, serde_json::to_string(&self)?)?)
    }
}
