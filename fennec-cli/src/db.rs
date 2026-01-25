pub mod battery_log;
pub mod key;
pub mod scalars;
pub mod selectable;

use std::path::Path;

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
    pub async fn connect(path: &Path, run_script: bool) -> Result<Self> {
        // FIXME: concurrent access from different processes. Play around with `ATTACH`?
        // info!(?path, "connecting to the database…");
        let connection = Builder::new_local(path.to_str().unwrap()).build().await?.connect()?;
        if run_script {
            connection.execute_batch(Self::SCRIPT).await?;
        }
        Ok(Self(connection))
    }
}
