pub mod battery_log;
pub mod key;
pub mod scalars;
pub mod selectable;

use std::path::Path;

use anyhow::Context;
use turso::{Builder, Connection};

use crate::prelude::*;

#[must_use]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Db(Connection);

impl Db {
    // language=sqlite
    const SCRIPT: &str = r"
        CREATE TABLE IF NOT EXISTS scalars (key TEXT NOT NULL PRIMARY KEY, value ANY);

        CREATE TABLE IF NOT EXISTS battery_logs (
            timestamp_millis    INTEGER NOT NULL PRIMARY KEY,
            residual_energy_kwh REAL NOT NULL,
            import_kwh          REAL NOT NULL,
            export_kwh          REAL NOT NULL
        );
    ";

    #[instrument(skip_all)]
    pub async fn connect(path: &Path) -> Result<Self> {
        info!(?path, "connecting to the database…");
        let connection = Builder::new_local(path.to_str().context("failed to convert the path")?)
            .build()
            .await?
            .connect()?;
        connection.execute_batch(Self::SCRIPT).await?;
        Ok(Self(connection))
    }
}
