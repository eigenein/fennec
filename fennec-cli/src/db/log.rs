use bson::doc;
use mongodb::{
    error::ErrorKind,
    options::{TimeseriesGranularity, TimeseriesOptions},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::{db::Db, prelude::*};

/// Anything that can be logged into a time-series collection.
pub trait Log: Send + Sync + Serialize + DeserializeOwned {
    const COLLECTION_NAME: &str;

    /// FIXME: make configurable.
    const EXPIRE_AFTER_SECONDS: u32 = 365 * 24 * 60 * 60;

    /// Initialize the underlying time-series collection.
    #[instrument(skip_all, fields(collection_name = Self::COLLECTION_NAME))]
    async fn initialize_time_series(db: &Db) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(TimeseriesGranularity::Minutes)
            .build();
        db.0.create_collection(Self::COLLECTION_NAME).timeseries(options).await.or_else(
            |error| match error.kind.as_ref() {
                ErrorKind::Command(error) if error.code == 48 => {
                    warn!("collection already exists");
                    Ok(())
                }
                _ => Err(error),
            },
        )?;
        db.0.run_command(doc! {
            "collMod": Self::COLLECTION_NAME,
            "expireAfterSeconds": Self::EXPIRE_AFTER_SECONDS,
        })
        .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn insert_into(&self, db: &Db) -> Result {
        info!(collection_name = Self::COLLECTION_NAME, "inserting the log…");
        db.0.collection::<Self>(Self::COLLECTION_NAME)
            .insert_one(self)
            .await
            .context("failed to insert the log")?;
        Ok(())
    }
}
