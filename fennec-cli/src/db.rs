use std::fmt::Debug;

use bson::{deserialize_from_document, doc, serialize_to_document};
use chrono::{DateTime, Local};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use mongodb::{
    Client,
    Database,
    error::ErrorKind,
    options::{ReturnDocument, TimeseriesOptions},
};

use crate::{db::state::ApplicationState, prelude::*};

pub mod battery;
pub mod consumption;
mod measurement;
pub mod state;

pub use self::measurement::Measurement;

#[must_use]
#[derive(Clone)]
pub struct Db {
    client: Client,
    inner: Database,
}

impl Db {
    const STATES_COLLECTION_NAME: &'static str = "states";

    /// Connect to the database with the specified URI.
    ///
    /// The URO *must* specify the database name.
    #[instrument(skip_all)]
    pub async fn with_uri(uri: impl AsRef<str> + Debug) -> Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let inner = client
            .default_database()
            .context("MongoDB URI does not define the default database")?;
        let this = Self { client, inner };
        this.initialize_time_series::<battery::Measurement>().await?;
        this.initialize_time_series::<consumption::Measurement>().await?;
        Ok(this)
    }

    #[instrument(skip_all)]
    pub async fn measurements<M: Measurement>(
        &self,
        since: DateTime<Local>,
    ) -> Result<impl TryStream<Ok = M, Error = Error>> {
        info!(collection_name = M::COLLECTION_NAME, ?since, "querying logs…");
        Ok(self
            .inner
            .collection::<M>(M::COLLECTION_NAME)
            .find(doc! { "timestamp": { "$gte": since } })
            .sort(doc! { "timestamp": 1 })
            .await
            .context("failed to query the battery logs")?
            .map_err(Error::from))
    }

    /// Retrieve the typed global state.
    #[instrument(skip_all, fields(id = ?S::ID))]
    #[expect(dead_code)]
    pub async fn get_application_state<S: ApplicationState>(&self) -> Result<Option<S>> {
        info!("fetching the state…");
        let filter = doc! { "_id": S::ID };
        self.inner
            .collection(Self::STATES_COLLECTION_NAME)
            .find_one(filter)
            .await
            .with_context(|| format!("failed to fetch `{:?}`", S::ID))?
            .map(deserialize_from_document)
            .transpose()
            .with_context(|| format!("failed to deserialize `{:?}`", S::ID))
    }

    /// Replace the typed global state and return the previous value.
    #[instrument(skip_all, fields(id = ?S::ID))]
    pub async fn set_application_state<S: ApplicationState>(&self, state: &S) -> Result<Option<S>> {
        info!("saving the state…");
        let filter = doc! { "_id": S::ID };
        let mut replacement = serialize_to_document(state)?;
        replacement.insert("_id", S::ID);
        let old_state = self
            .inner
            .collection(Self::STATES_COLLECTION_NAME)
            .find_one_and_replace(filter, replacement)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await
            .with_context(|| format!("failed to upsert `{:?}`", S::ID))?;
        old_state.map(deserialize_from_document).transpose().context("failed to upsert the state")
    }

    pub async fn shutdown(self) {
        self.client.shutdown().await;
    }

    /// Initialize the underlying time-series collection.
    #[instrument(skip_all, fields(collection_name = M::COLLECTION_NAME))]
    async fn initialize_time_series<M: Measurement>(&self) -> Result {
        let options = TimeseriesOptions::builder()
            .time_field("timestamp")
            .granularity(M::GRANULARITY)
            .build();
        self.inner.create_collection(M::COLLECTION_NAME).timeseries(options).await.or_else(
            |error| match error.kind.as_ref() {
                ErrorKind::Command(error) if error.code == 48 => {
                    warn!("collection already exists");
                    Ok(())
                }
                _ => Err(error),
            },
        )?;
        self.inner
            .run_command(doc! {
                "collMod": M::COLLECTION_NAME,
                "expireAfterSeconds": M::EXPIRE_AFTER_SECONDS,
            })
            .await?;
        Ok(())
    }
}
