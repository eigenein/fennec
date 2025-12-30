use std::path::Path;

use anyhow::Context;
use chrono::{DateTime, Local, TimeDelta};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    api::home_assistant::{EnergyAttributes, EnergyState},
    core::series::{Aggregate, Differentiate},
    prelude::{instrument, *},
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Statistics {
    #[serde(default)]
    pub generated_at: DateTime<Local>,

    pub household: HouseholdParameters,

    pub battery: BatteryParameters,
}

impl Statistics {
    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn read_from(path: &Path) -> crate::prelude::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read statistics from `{}`", path.display()))?;
        toml::from_str(&contents).context("failed to deserialize the statistics")
    }

    #[instrument(skip_all, fields(path = %path.display()))]
    pub fn write_to(&self, path: &Path) -> crate::prelude::Result {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write the statistics to `{}`", path.display()))
    }
}

impl FromIterator<EnergyState> for Statistics {
    fn from_iter<T: IntoIterator<Item = EnergyState>>(iterator: T) -> Self {
        info!("Crunching numbers…");
        let series = iterator.into_iter().map(|state| (state.last_changed_at, state)).collect_vec();
        let hourly_stand_by_power = series
            .iter()
            .map(|(timestamp, energy_state)| (*timestamp, energy_state.net_consumption))
            .deltas()
            .filter(|(interval, _)| interval.end > interval.start)
            .differentiate()
            .median_hourly();
        Self {
            generated_at: Local::now(),
            household: HouseholdParameters { hourly_stand_by_power },
            battery: series.into_iter().collect(),
        }
    }
}

impl EnergyAttributes {
    pub fn is_importing(&self) -> bool {
        self.import >= KilowattHours::from(0.001)
    }

    pub fn is_exporting(&self) -> bool {
        self.export >= KilowattHours::from(0.001)
    }

    pub fn is_idling(&self) -> bool {
        !self.is_importing() && !self.is_exporting() && self.residual_energy <= KilowattHours::ZERO
    }

    pub fn is_charging(&self) -> bool {
        self.is_importing()
            && !self.is_exporting()
            && self.residual_energy >= KilowattHours::ONE_THOUSANDTH
    }

    pub fn is_discharging(&self) -> bool {
        self.is_exporting()
            && !self.is_importing()
            && self.residual_energy <= -KilowattHours::ONE_THOUSANDTH
    }

    pub fn as_charging_efficiency(&self) -> f64 {
        (self.residual_energy / (self.import - self.export)).0
    }

    pub fn as_discharging_efficiency(&self) -> f64 {
        ((self.import - self.export) / self.residual_energy).0
    }
}

#[derive(Copy, Clone, derive_more::Add, derive_more::Sum)]
struct Delta {
    time: TimeDelta,
    energy: EnergyAttributes,
}

impl Delta {
    pub fn as_parasitic_load(&self) -> Kilowatts {
        (self.energy.export - self.energy.import - self.energy.residual_energy) / self.time
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct HouseholdParameters {
    #[serde(rename = "hourly_stand_by_power_kilowatts")]
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}

#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct BatteryParameters {
    #[serde(rename = "parasitic_load_kilowatts")]
    pub parasitic_load: Kilowatts,

    pub charging_efficiency: f64,

    pub discharging_efficiency: f64,
}

impl FromIterator<(DateTime<Local>, EnergyState)> for BatteryParameters {
    /// Analyse battery parameters by the energy state history.
    ///
    /// FIXME: properly handle the panics.
    fn from_iter<T: IntoIterator<Item = (DateTime<Local>, EnergyState)>>(iterator: T) -> Self {
        let battery_deltas = iterator
            .into_iter()
            .map(|(timestamp, energy_state)| (timestamp, energy_state.attributes))
            .deltas()
            .filter(|(interval, delta)| {
                (interval.end > interval.start)
                    && (delta.import >= KilowattHours::ZERO)
                    && (delta.export >= KilowattHours::ZERO)
            })
            .map(|(time_range, delta)| Delta {
                time: time_range.end - time_range.start,
                energy: delta,
            })
            .collect_vec();
        info!(count = battery_deltas.len(), "Collected battery delta's");
        assert_ne!(battery_deltas.len(), 0);

        let idling_delta =
            battery_deltas.iter().filter(|point| point.energy.is_idling()).copied().sum::<Delta>();
        assert_ne!(idling_delta.time, TimeDelta::zero());
        let parasitic_load = idling_delta.as_parasitic_load();
        info!(
            ?parasitic_load,
            idling_hours = idling_delta.time.as_seconds_f64() / 3600.0,
            residual_energy_delta = ?idling_delta.energy.residual_energy,
            import = ?idling_delta.energy.import,
            export = ?idling_delta.energy.export,
        );

        let mut charging_samples = Vec::new();
        let mut discharging_samples = Vec::new();
        for mut delta in battery_deltas {
            delta.energy.residual_energy += parasitic_load * delta.time;
            if delta.energy.is_charging() {
                charging_samples.push(OrderedFloat(delta.energy.as_charging_efficiency()));
            } else if delta.energy.is_discharging() {
                discharging_samples.push(OrderedFloat(delta.energy.as_discharging_efficiency()));
            }
        }
        let n_charging_samples = charging_samples.len();
        let charging_efficiency = charging_samples.median().unwrap();
        let n_discharging_samples = discharging_samples.len();
        let discharging_efficiency = discharging_samples.median().unwrap();
        info!(coefficient = ?charging_efficiency, n_samples = n_charging_samples);
        info!(coefficient = ?discharging_efficiency, n_samples = n_discharging_samples);
        let this = Self {
            parasitic_load,
            charging_efficiency: charging_efficiency.0,
            discharging_efficiency: discharging_efficiency.0,
        };
        info!(round_trip_efficiency = ?this.round_trip_efficiency());
        this
    }
}

impl BatteryParameters {
    pub fn round_trip_efficiency(&self) -> f64 {
        self.charging_efficiency * self.discharging_efficiency
    }
}
