use std::fmt::Debug;

use bson::{deserialize_from_document, doc, serialize_to_document};
use chrono::{DateTime, Local};
use futures_core::TryStream;
use futures_util::TryStreamExt;
use mongodb::{Client, Database, options::ReturnDocument};

use crate::{db::state::ApplicationState, prelude::*};

pub mod battery;
pub mod consumption;
mod log;
pub mod state;

pub use self::log::TimeSeries;

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
        battery::LogEntry::initialize(&this).await?;
        consumption::LogEntry::initialize(&this).await?;
        Ok(this)
    }

    #[instrument(skip_all)]
    pub async fn find_logs<L: TimeSeries>(
        &self,
        since: DateTime<Local>,
    ) -> Result<impl TryStream<Ok = L, Error = Error>> {
        info!(collection_name = L::COLLECTION_NAME, ?since, "querying logs…");
        Ok(self
            .inner
            .collection::<L>(L::COLLECTION_NAME)
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
}
