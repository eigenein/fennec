pub mod measurement;
pub mod measurements;
pub mod scalars;
pub mod selectable;

use std::path::Path;

use anyhow::Context;
use turso::{
    Builder,
    Connection,
    Value,
    transaction::{Transaction, TransactionBehavior},
};

use crate::{db::scalars::Scalars, prelude::*};

#[must_use]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Db(Connection);

impl Db {
    const VERSION_KEY: &str = "schema_version";

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

    #[instrument(skip_all, ret)]
    pub async fn get_version(&self) -> Result<i64> {
        Ok(Scalars(self).select::<Option<i64>>(Self::VERSION_KEY).await?.unwrap_or_default())
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
                "CREATE TABLE measurements (
                    timestamp_millis   INTEGER NOT NULL PRIMARY KEY,
                    total_import_kwh   REAL NOT NULL,
                    total_export_kwh   REAL NOT NULL,
                    battery_import_kwh REAL NOT NULL,
                    battery_export_kwh REAL NOT NULL,
                    battery_energy_kwh REAL NOT NULL
                )",
            ),
        ];

        let current_version = self.get_version().await?;
        info!(current_version, "checking migrations…");

        for (version, sql) in MIGRATIONS {
            if *version > current_version {
                info!(version, "applying migration…");
                let tx = Transaction::new(self, TransactionBehavior::Deferred).await?;
                tx.execute_batch(sql).await?;
                Scalars(&tx).upsert(Self::VERSION_KEY, Value::Integer(*version)).await?;
                tx.commit().await?;
            }
        }
        Ok(())
    }
}
