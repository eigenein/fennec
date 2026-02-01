use bson::{Document, doc, serialize_to_bson, serialize_to_document};
use mongodb::Collection;
use serde::Serialize;

use crate::{
    prelude::*,
    quantity::{energy::MilliwattHours, power::Kilowatts},
};

/// State `_id` in the database.
#[derive(Copy, Clone, Debug, Serialize)]
pub enum StateId {
    #[serde(rename = "batteryResidualEnergy")]
    BatteryResidualEnergy,

    #[serde(rename = "hourlyStandByPower")]
    HourlyStandByPower,
}

pub trait State: Serialize {
    const ID: StateId;
}

/// Last known battery residual energy.
#[must_use]
#[derive(Copy, Clone, Serialize)]
pub struct BatteryResidualEnergy {
    #[serde(rename = "milliwattHours")]
    residual_energy: MilliwattHours,
}

impl From<MilliwattHours> for BatteryResidualEnergy {
    fn from(residual_energy: MilliwattHours) -> Self {
        Self { residual_energy }
    }
}

impl State for BatteryResidualEnergy {
    const ID: StateId = StateId::BatteryResidualEnergy;
}

#[must_use]
#[derive(Copy, Clone, Serialize)]
pub struct HourlyStandByPower {
    #[serde(rename = "kilowatts")]
    hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl From<[Option<Kilowatts>; 24]> for HourlyStandByPower {
    fn from(hourly_stand_by_power: [Option<Kilowatts>; 24]) -> Self {
        Self { hourly_stand_by_power }
    }
}

impl State for HourlyStandByPower {
    const ID: StateId = StateId::HourlyStandByPower;
}

/// Collection that contains current states preserved between the application runs.
#[must_use]
pub struct States(pub(super) Collection<Document>);

impl States {
    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn upsert<S: State>(&self, state: &S) -> Result {
        let id = serialize_to_bson(&S::ID)?;
        let filter = doc! { "_id": &id };
        let mut replacement = serialize_to_document(state)?;
        replacement.insert("_id", id);
        self.0
            .replace_one(filter, replacement)
            .upsert(true)
            .await
            .with_context(|| format!("failed to upsert `{:?}`", S::ID))?;
        Ok(())
    }
}
