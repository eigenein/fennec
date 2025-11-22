use std::path::Path;

use chrono::{DateTime, Local, TimeDelta};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::{
    api::home_assistant::{EnergyAttributes, EnergyState},
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

    pub battery: Option<Battery>,
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
        let series = iterator.into_iter().map(|state| (state.last_changed_at, state)).collect_vec();

        let hourly_stand_by_power = series
            .iter()
            .map(|(timestamp, energy_state)| {
                (*timestamp, energy_state.net_consumption - energy_state.attributes.solar_yield)
            })
            .deltas()
            .filter(|(time_span, _)| time_span.end > time_span.start)
            .differentiate()
            .median_hourly();

        let battery_deltas = series
            .into_iter()
            .map(|(timestamp, energy_state)| (timestamp, energy_state.attributes))
            .deltas()
            .filter(|(time_span, delta)| {
                (time_span.end > time_span.start)
                    && (delta.battery_energy_import >= KilowattHours::ZERO)
                    && (delta.battery_energy_export >= KilowattHours::ZERO)
            })
            .map(|(time_range, delta)| Delta {
                time: time_range.end - time_range.start,
                energy: delta,
            })
            .collect_vec();
        info!(count = battery_deltas.len(), "Collected battery delta's");

        let parasitic_load = battery_deltas
            .iter()
            .filter(|point| point.energy.is_idling())
            .copied()
            .sum::<Delta>()
            .as_parasitic_load();
        info!(?parasitic_load, "Calculated");

        let mut charging_samples = Vec::new();
        let mut discharging_samples = Vec::new();
        for mut delta in battery_deltas {
            delta.energy.battery_residual_energy += parasitic_load * delta.time;
            if delta.energy.is_charging() {
                charging_samples.push(delta.energy.as_charging_efficiency());
            } else if delta.energy.is_discharging() {
                discharging_samples.push(delta.energy.as_discharging_efficiency());
            }
        }
        let n_charging_samples = charging_samples.len();
        let charging_efficiency = charging_samples.median();
        let n_discharging_samples = discharging_samples.len();
        let discharging_efficiency = discharging_samples.median();
        info!(coefficient = ?charging_efficiency, n_samples = n_charging_samples, "Calculated charging efficiency");
        info!(coefficient = ?discharging_efficiency, n_samples = n_discharging_samples, "Calculated charging efficiency");

        Self {
            generated_at: Some(Local::now()),
            household: Household { hourly_stand_by_power },
            battery: Some(Battery { parasitic_load }),
        }
    }
}

impl EnergyAttributes {
    pub fn is_importing(&self) -> bool {
        self.battery_energy_import >= KilowattHours::from(0.001)
    }

    pub fn is_exporting(&self) -> bool {
        self.battery_energy_export >= KilowattHours::from(0.001)
    }

    pub fn is_idling(&self) -> bool {
        !self.is_importing()
            && !self.is_exporting()
            && self.battery_residual_energy <= KilowattHours::ZERO
    }

    pub fn is_charging(&self) -> bool {
        self.is_importing()
            && !self.is_exporting()
            && self.battery_residual_energy >= KilowattHours::ONE_THOUSANDTH
    }

    pub fn is_discharging(&self) -> bool {
        self.is_exporting()
            && !self.is_importing()
            && self.battery_residual_energy <= -KilowattHours::ONE_THOUSANDTH
    }

    pub fn as_charging_efficiency(&self) -> OrderedFloat<f64> {
        self.battery_residual_energy / (self.battery_energy_import - self.battery_energy_export)
    }

    pub fn as_discharging_efficiency(&self) -> OrderedFloat<f64> {
        (self.battery_energy_import - self.battery_energy_export) / self.battery_residual_energy
    }
}

#[derive(Copy, Clone, derive_more::Add, derive_more::Sum)]
struct Delta {
    time: TimeDelta,
    energy: EnergyAttributes,
}

impl Delta {
    pub fn as_parasitic_load(&self) -> Kilowatts {
        (self.energy.battery_energy_export
            - self.energy.battery_energy_import
            - self.energy.battery_residual_energy)
            / self.time
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Household {
    #[serde(rename = "hourly_stand_by_power_kilowatts")]
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Battery {
    #[serde(rename = "parasitic_load_kilowatts")]
    pub parasitic_load: Kilowatts,
}
