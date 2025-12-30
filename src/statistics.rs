pub mod energy;

use std::path::Path;

use anyhow::Context;
use bon::Builder;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::{instrument, *},
    statistics::energy::EnergyStatistics,
};

#[must_use]
#[derive(Builder, Serialize, Deserialize)]
pub struct Statistics {
    #[serde(default)]
    #[builder(default = chrono::Local::now())]
    pub generated_at: DateTime<Local>,

    #[serde(flatten)]
    pub energy: EnergyStatistics,
}

impl<S: statistics_builder::IsComplete> StatisticsBuilder<S> {
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn write_to(self, path: &Path) -> Result {
        let contents = toml::to_string_pretty(&self.build())?;
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write the statistics to `{}`", path.display()))
    }
}

impl Statistics {
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn read_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read statistics from `{}`", path.display()))?;
        toml::from_str(&contents).context("failed to deserialize the statistics")
    }
}
