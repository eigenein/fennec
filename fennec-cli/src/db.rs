use std::{fmt::Debug, time::Duration};

use bson::doc;
use futures_core::TryStream;
use futures_util::TryStreamExt;
use mongodb::{Client, Database, error::ErrorKind, options::TimeseriesOptions};

use crate::{db::commands::set_expiration_time, prelude::*};

mod commands;
mod measurement;
pub mod power;

pub use self::measurement::Measurement;

#[must_use]
#[derive(Clone)]
pub struct Db {
    client: Client,
    inner: Database,
}

impl Db {
    /// Connect to the database with the specified URI.
    ///
    /// The URI *must* specify the database name.
    #[instrument(skip_all)]
    pub async fn with_uri(uri: impl AsRef<str> + Debug) -> Result<Self> {
        info!("connecting…");
        let client = Client::with_uri_str(uri).await?;
        let inner = client
            .default_database()
            .context("MongoDB URI does not define the default database")?;
        let this = Self { client, inner };
        this.create_time_series::<power::Measurement>().await?;
        Ok(this)
    }

    /// Set expiration time for the time series.
    #[instrument(skip_all, fields(expiration_time = ?expiration_time))]
    pub async fn set_expiration_time<M: Measurement>(&self, expiration_time: Duration) -> Result {
        self.inner.run_command(set_expiration_time::<M>(expiration_time)?).await?;
        Ok(())
    }

    /// Read all the persisted measurements.
    ///
    /// The measurements get evicted by the database based on the set expiration time.
    #[instrument(skip_all)]
    pub async fn measurements<M: Measurement>(
        &self,
    ) -> Result<impl TryStream<Ok = M, Error = Error>> {
        info!(collection_name = M::COLLECTION_NAME, "querying logs…");
        Ok(self
            .inner
            .collection::<M>(M::COLLECTION_NAME)
            .find(doc! {})
            .sort(doc! { "timestamp": 1 })
            .await
            .context("failed to query the battery logs")?
            .map_err(Error::from))
    }

    pub async fn shutdown(self) {
        self.client.shutdown().await;
    }

    /// Initialize the underlying time-series collection.
    #[instrument(skip_all, fields(collection_name = M::COLLECTION_NAME))]
    async fn create_time_series<M: Measurement>(&self) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(M::GRANULARITY)
            .build();
        self.inner.create_collection(M::COLLECTION_NAME).timeseries(options).await.or_else(
            |error| match *error.kind {
                ErrorKind::Command(error) if error.code == 48 => {
                    info!("collection already exists");
                    Ok(())
                }
                _ => Err(error),
            },
        )?;
        Ok(())
    }
}
