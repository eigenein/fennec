pub mod key;
pub mod measurement;
pub mod measurements;
pub mod scalars;
pub mod selectable;
pub mod transition;
pub mod transitions;

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

        CREATE TABLE IF NOT EXISTS measurements (
            timestamp_millis   INTEGER NOT NULL PRIMARY KEY,
            total_import_kwh   REAL NOT NULL,
            total_export_kwh   REAL NOT NULL,
            battery_import_kwh REAL NOT NULL,
            battery_export_kwh REAL NOT NULL,
            battery_energy_kwh REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS residual_energy_transitions (
            timestamp_millis INTEGER NOT NULL PRIMARY KEY,
            milliwatt_hours  INTEGER NOT NULL,
            FOREIGN KEY (timestamp_millis) REFERENCES measurements(timestamp_millis)
        );
    ";

    #[instrument]
    pub async fn connect(path: &Path) -> Result<Self> {
        let connection = Builder::new_local(path.to_str().context("failed to convert the path")?)
            .build()
            .await?
            .connect()?;
        connection.execute_batch(Self::SCRIPT).await?;
        Ok(Self(connection))
    }
}
