pub mod key;
pub mod measurement;
pub mod measurements;
pub mod scalars;
pub mod selectable;
pub mod transition;
pub mod transitions;

use std::path::Path;

use anyhow::Context;
use turso::{
    Builder,
    Connection,
    transaction::{Transaction, TransactionBehavior},
};

use crate::{
    db::{key::Key, scalars::Scalars},
    prelude::*,
};

#[must_use]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Db(Connection);

impl Db {
    #[instrument]
    pub async fn connect(path: &Path) -> Result<Self> {
        let connection = Builder::new_local(path.to_str().context("failed to convert the path")?)
            .build()
            .await?
            .connect()?;
        let mut db = Self(connection);
        db.create_scalars().await?;
        db.migrate().await?;
        Ok(db)
    }

    #[instrument(skip_all)]
    async fn create_scalars(&self) -> Result {
        // language=sqlite
        const SQL: &str =
            "CREATE TABLE IF NOT EXISTS scalars (key TEXT NOT NULL PRIMARY KEY, value ANY)";
        self.0.execute(SQL, ()).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn migrate(&mut self) -> Result {
        const MIGRATIONS: &[(i64, &str)] = &[
            // language=sqlite
            (
                1,
                "
                    CREATE TABLE measurements (
                        timestamp_millis   INTEGER NOT NULL PRIMARY KEY,
                        total_import_kwh   REAL NOT NULL,
                        total_export_kwh   REAL NOT NULL,
                        battery_import_kwh REAL NOT NULL,
                        battery_export_kwh REAL NOT NULL,
                        battery_energy_kwh REAL NOT NULL
                )",
            ),
            // language=sqlite
            (
                2,
                "
                    CREATE TABLE residual_energy_transitions (
                        timestamp_millis INTEGER NOT NULL PRIMARY KEY,
                        milliwatt_hours  INTEGER NOT NULL,
                        FOREIGN KEY (timestamp_millis) REFERENCES measurements(timestamp_millis)
                    )
                ",
            ),
        ];

        let current_version =
            Scalars(self).select_scalar::<i64>(Key::SchemaVersion).await?.unwrap_or(0);
        info!(current_version, "checking migrations…");

        for (version, sql) in MIGRATIONS {
            if *version > current_version {
                info!(version, "applying migration…");
                let tx = Transaction::new(self, TransactionBehavior::Deferred).await?;
                tx.execute_batch(sql).await?;
                Scalars(&tx).upsert(Key::SchemaVersion, *version).await?;
                tx.commit().await?;
            }
        }
        Ok(())
    }
}
