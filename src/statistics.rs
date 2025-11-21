use std::path::Path;

use chrono::{DateTime, Local};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::{
    api::home_assistant::EnergyState,
    core::series::{Aggregate, Differentiate},
    prelude::*,
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Statistics {
    #[serde(default)]
    pub generated_at: Option<DateTime<Local>>,

    pub household: Household,
}

impl Statistics {
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn read_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read statistics from `{}`", path.display()))?;
        toml::from_str(&contents).context("failed to deserialize the statistics")
    }

    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn write_to(&self, path: &Path) -> Result {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write the statistics to `{}`", path.display()))
    }
}

impl FromIterator<EnergyState> for Statistics {
    #[instrument(skip_all)]
    fn from_iter<T: IntoIterator<Item = EnergyState>>(iterator: T) -> Self {
        info!("Crunching numbers…");
        let mut hourly_stand_by_power = iterator
            .into_iter()
            .map(|state| {
                (
                    state.last_changed_at,
                    state.net_consumption
                        - state.attributes.solar_yield.unwrap_or(KilowattHours::ZERO),
                )
            })
            .deltas()
            .differentiate()
            .median_hourly();
        for kilowatts in hourly_stand_by_power.iter_mut().flatten() {
            // Round the power to watts to remove the awkward number of decimal points:
            kilowatts.0 = OrderedFloat((kilowatts.0 * 1000.0).round() / 1000.0);
        }
        Self { generated_at: Some(Local::now()), household: Household { hourly_stand_by_power } }
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Household {
    #[serde(rename = "hourly_stand_by_power_kilowatts")]
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}
