use bson::{Document, deserialize_from_document, doc, serialize_to_bson, serialize_to_document};
use derive_more::{From, Into};
use mongodb::Collection;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

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

pub trait State: Serialize + DeserializeOwned {
    const ID: StateId;
}

/// Last known battery residual energy.
#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize, From)]
pub struct BatteryResidualEnergy {
    #[serde(rename = "milliwattHours")]
    residual_energy: MilliwattHours,
}

impl State for BatteryResidualEnergy {
    const ID: StateId = StateId::BatteryResidualEnergy;
}

#[must_use]
#[derive(Copy, Clone, Default, Serialize, Deserialize, From, Into)]
pub struct HourlyStandByPower {
    #[serde(rename = "kilowatts")]
    hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl State for HourlyStandByPower {
    const ID: StateId = StateId::HourlyStandByPower;
}

/// Collection that contains current states preserved between the application runs.
#[must_use]
pub struct States(pub(super) Collection<Document>);

impl States {
    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn get<S: State>(&self) -> Result<Option<S>> {
        let filter = doc! { "_id": serialize_to_bson(&S::ID)? };
        self.0
            .find_one(filter)
            .await
            .with_context(|| format!("failed to fetch `{:?}`", S::ID))?
            .map(deserialize_from_document)
            .transpose()
            .with_context(|| format!("failed to deserialize `{:?}`", S::ID))
    }

    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn upsert<S: State>(&self, state: &S) -> Result {
        info!("saving the state…");
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
