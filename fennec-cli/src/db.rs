use std::fmt::Debug;

use bson::doc;
use mongodb::{Client, ClientSession, Database, error::ErrorKind, options::TimeseriesOptions};

use crate::{
    db::{battery_log::BatteryLogs, state::States},
    prelude::*,
};

pub mod battery_log;
pub mod state;

#[must_use]
pub struct Db {
    client: Client,
    inner: Database,
}

impl Db {
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
        let mut session = this.start_session().await?;
        BatteryLogs::initialize_on(&mut session).await?;
        Ok(this)
    }

    pub async fn start_session(&self) -> Result<SessionDb> {
        let session = self.client.start_session().await.context("failed to start a session")?;
        Ok(SessionDb { inner: self.inner.clone(), session })
    }
}

pub struct SessionDb {
    inner: Database,
    session: ClientSession,
}

impl SessionDb {
    #[instrument(skip_all, fields(name = name))]
    pub(self) async fn create_timeseries(
        &mut self,
        name: &str,
        options: TimeseriesOptions,
    ) -> Result {
        self.inner
            .create_collection(name)
            .timeseries(options)
            .session(&mut self.session)
            .await
            .or_else(|error| match error.kind.as_ref() {
                ErrorKind::Command(error) if error.code == 48 => {
                    warn!("collection already exists");
                    Ok(())
                }
                _ => Err(error),
            })?;
        self.inner
            .run_command(doc! {
                "collMod": name,
                "expireAfterSeconds": 365 * 24 * 60 * 60, // FIXME: make configurable.
            })
            .await?;
        Ok(())
    }

    pub const fn session(&mut self) -> &mut ClientSession {
        &mut self.session
    }

    pub fn states(&mut self) -> States<'_> {
        States {
            collection: self.inner.collection(States::COLLECTION_NAME),
            session: &mut self.session,
        }
    }

    pub fn battery_logs(&mut self) -> BatteryLogs<'_> {
        BatteryLogs {
            collection: self.inner.collection(BatteryLogs::COLLECTION_NAME),
            session: &mut self.session,
        }
    }
}
