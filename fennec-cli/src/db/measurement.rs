use mongodb::options::TimeseriesGranularity;
use serde::{Serialize, de::DeserializeOwned};

use crate::{db::Db, prelude::*};

pub trait Measurement: Send + Sync + Serialize + DeserializeOwned {
    const COLLECTION_NAME: &str;
    const GRANULARITY: TimeseriesGranularity;

    #[instrument(skip_all)]
    async fn insert_into(&self, db: &Db) -> Result {
        db.inner
            .collection::<Self>(Self::COLLECTION_NAME)
            .insert_one(self)
            .await
            .context("failed to insert the measurement")?;
        Ok(())
    }
}
