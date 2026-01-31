pub mod battery_log;
pub mod key;
pub mod scalars;
pub mod selectable;

use std::path::Path;

use turso::{Builder, Connection};

use crate::prelude::*;

#[must_use]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct LegacyDb(Connection);

impl LegacyDb {
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
    pub async fn connect(path: &Path, run_script: bool) -> Result<Self> {
        // FIXME: <https://github.com/tursodatabase/turso/issues/769>.
        let connection = Builder::new_local(path.to_str().unwrap()).build().await?.connect()?;
        if run_script {
            // TODO: move to a separate function and make it an optional flag in `DatabaseArgs`.
            connection.execute_batch(Self::SCRIPT).await?;
        }
        Ok(Self(connection))
    }
}
