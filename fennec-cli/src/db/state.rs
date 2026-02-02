use bson::{Document, deserialize_from_document, doc, serialize_to_document};
use derive_more::{From, Into};
use mongodb::{Collection, options::ReturnDocument};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    db::Db,
    prelude::*,
    quantity::{energy::MilliwattHours, power::Kilowatts},
};

pub trait State: Serialize + DeserializeOwned {
    const ID: &str;
}

/// Last known battery residual energy.
#[must_use]
#[derive(Copy, Clone, Serialize, Deserialize, From, Into)]
pub struct BatteryResidualEnergy {
    #[serde(rename = "milliwattHours")]
    residual_energy: MilliwattHours,
}

impl State for BatteryResidualEnergy {
    const ID: &str = "batteryResidualEnergy";
}

#[must_use]
#[derive(Copy, Clone, Default, Serialize, Deserialize, From, Into)]
pub struct HourlyStandByPower {
    #[serde(rename = "kilowatts")]
    hourly_stand_by_power: [Option<Kilowatts>; 24],
}

impl State for HourlyStandByPower {
    const ID: &str = "hourlyStandByPower";
}

/// Collection that contains current states preserved between the application runs.
#[must_use]
pub struct States(Collection<Document>);

impl From<&Db> for States {
    fn from(db: &Db) -> Self {
        Self(db.0.collection(Self::COLLECTION_NAME))
    }
}

impl States {
    const COLLECTION_NAME: &'static str = "states";

    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn get<S: State>(&self) -> Result<Option<S>> {
        info!("fetching the state…");
        let filter = doc! { "_id": S::ID };
        self.0
            .find_one(filter)
            .await
            .with_context(|| format!("failed to fetch `{:?}`", S::ID))?
            .map(deserialize_from_document)
            .transpose()
            .with_context(|| format!("failed to deserialize `{:?}`", S::ID))
    }

    /// Replace the state and return the previous value.
    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn set<S: State>(&self, state: &S) -> Result<Option<S>> {
        info!("saving the state…");
        let filter = doc! { "_id": S::ID };
        let mut replacement = serialize_to_document(state)?;
        replacement.insert("_id", S::ID);
        let old_state = self
            .0
            .find_one_and_replace(filter, replacement)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await
            .with_context(|| format!("failed to upsert `{:?}`", S::ID))?;
        old_state.map(deserialize_from_document).transpose().context("failed to upsert the state")
    }
}
