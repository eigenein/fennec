use chrono::{DateTime, Local, TimeDelta};
use itertools::Itertools;
use serde::{Deserialize, Serialize, Serializer};
use tracing::info;

use crate::{
    api::home_assistant::{EnergyAttributes, EnergyState},
    core::series::{Aggregate, Differentiate},
    quantity::{energy::KilowattHours, power::Kilowatts},
};

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct EnergyStatistics {
    pub household: HouseholdParameters,

    pub battery: BatteryEfficiencyParameters,
}

impl FromIterator<EnergyState> for EnergyStatistics {
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
            household: HouseholdParameters { hourly_stand_by_power },
            battery: series.into_iter().collect(),
        }
    }
}

impl EnergyAttributes {
    #[must_use]
    pub const fn is_importing(&self) -> bool {
        self.import.is_significant()
    }

    #[must_use]
    pub const fn is_exporting(&self) -> bool {
        self.export.is_significant()
    }

    #[must_use]
    pub const fn is_idling(&self) -> bool {
        !self.is_importing() && !self.is_exporting()
    }

    #[must_use]
    pub const fn is_charging(&self) -> bool {
        self.is_importing() && !self.is_exporting()
    }

    #[must_use]
    pub const fn is_discharging(&self) -> bool {
        self.is_exporting() && !self.is_importing()
    }

    #[must_use]
    pub fn as_charging_efficiency(&self) -> f64 {
        self.residual_energy / (self.import - self.export)
    }

    #[must_use]
    pub fn as_discharging_efficiency(&self) -> f64 {
        (self.import - self.export) / self.residual_energy
    }
}

#[derive(Copy, Clone, derive_more::Add, derive_more::Sum)]
#[must_use]
struct Delta {
    time: TimeDelta,
    energy: EnergyAttributes,
}

impl Delta {
    pub fn as_parasitic_load(&self) -> Kilowatts {
        (self.energy.export - self.energy.import - self.energy.residual_energy) / self.time
    }

    pub fn without_parasitic_load(mut self, parasitic_load: Kilowatts) -> Self {
        self.energy.residual_energy += parasitic_load * self.time;
        self
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct HouseholdParameters {
    #[serde(
        rename = "hourly_stand_by_power_kilowatts",
        serialize_with = "HouseholdParameters::serialize_hourly_stand_by_power"
    )]
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl HouseholdParameters {
    /// Serialize the array with the items rounded to watts.
    fn serialize_hourly_stand_by_power<S: Serializer>(
        hourly_stand_by_power: &[Option<Kilowatts>],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            hourly_stand_by_power.iter().map(|kilowatts| kilowatts.map(Kilowatts::round_to_watts)),
        )
    }
}

#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct BatteryEfficiencyParameters {
    #[serde(
        rename = "parasitic_load_kilowatts",
        serialize_with = "BatteryEfficiencyParameters::serialize_kilowatts"
    )]
    pub parasitic_load: Kilowatts,

    #[serde(serialize_with = "BatteryEfficiencyParameters::serialize_efficiency")]
    pub charging_efficiency: f64,

    #[serde(serialize_with = "BatteryEfficiencyParameters::serialize_efficiency")]
    pub discharging_efficiency: f64,
}

impl BatteryEfficiencyParameters {
    #[expect(clippy::trivially_copy_pass_by_ref)]
    fn serialize_kilowatts<S: Serializer>(
        kilowatts: &Kilowatts,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        kilowatts.round_to_watts().serialize(serializer)
    }

    #[expect(clippy::trivially_copy_pass_by_ref)]
    fn serialize_efficiency<S: Serializer>(
        efficiency: &f64,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64((efficiency * 1000.0).round() / 1000.0)
    }
}

impl FromIterator<(DateTime<Local>, EnergyState)> for BatteryEfficiencyParameters {
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

        let charging_delta = battery_deltas
            .iter()
            .filter(|point| point.energy.is_charging())
            .map(|delta| delta.without_parasitic_load(parasitic_load))
            .sum::<Delta>();
        let charging_efficiency = charging_delta.energy.as_charging_efficiency();
        info!(
            ?charging_efficiency,
            charging_hours = charging_delta.time.as_seconds_f64() / 3600.0,
            residual_energy_delta = ?charging_delta.energy.residual_energy,
            import = ?charging_delta.energy.import,
            export = ?charging_delta.energy.export,
        );

        let discharging_delta = battery_deltas
            .iter()
            .filter(|point| point.energy.is_discharging())
            .map(|delta| delta.without_parasitic_load(parasitic_load))
            .sum::<Delta>();
        let discharging_efficiency = discharging_delta.energy.as_discharging_efficiency();
        info!(
            ?discharging_efficiency,
            discharging_hours = discharging_delta.time.as_seconds_f64() / 3600.0,
            residual_energy_delta = ?discharging_delta.energy.residual_energy,
            import = ?discharging_delta.energy.import,
            export = ?discharging_delta.energy.export,
        );

        let this = Self { parasitic_load, charging_efficiency, discharging_efficiency };
        info!(round_trip_efficiency = ?this.round_trip_efficiency());
        this
    }
}

impl BatteryEfficiencyParameters {
    pub fn round_trip_efficiency(&self) -> f64 {
        self.charging_efficiency * self.discharging_efficiency
    }
}
