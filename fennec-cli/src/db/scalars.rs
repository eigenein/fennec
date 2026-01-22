use turso::{Connection, Value};

use crate::prelude::*;

#[must_use]
pub struct Scalars<'c>(pub &'c Connection);

impl Scalars<'_> {
    #[instrument(skip_all, fields(key = key))]
    pub async fn select(&self, key: &str) -> Result<Option<Value>> {
        // language=sqlite
        const SQL: &str = "SELECT value FROM scalars WHERE key = ?1";
        match self.0.prepare_cached(SQL).await?.query_row((key,)).await {
            Ok(row) => Ok(Some(row.get_value(0)?)),
            Err(turso::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(anyhow::format_err!(error)),
        }
    }

    #[instrument(skip_all, fields(key = key))]
    pub async fn select_integer(&self, key: &str) -> Result<Option<i64>> {
        Ok(self.select(key).await?.and_then(|value| value.as_integer().copied()))
    }

    #[instrument(skip_all, fields(key = key))]
    pub async fn upsert(&self, key: &str, value: Value) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO scalars (key, value) VALUES (?1, ?2)
            ON CONFLICT DO UPDATE SET value = ?2
        ";
        self.0.prepare_cached(SQL).await?.execute((key, value)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::db::Db;

    #[tokio::test]
    async fn scalars_ok() -> Result {
        let db = Db::connect(Path::new(":memory:")).await?;
        assert_eq!(Scalars(&db).select("key").await?, None);
        Scalars(&db).upsert("key", Value::Integer(42)).await?;
        assert_eq!(Scalars(&db).select("key").await?, Some(Value::Integer(42)));
        Scalars(&db).upsert("key", Value::Integer(43)).await?;
        assert_eq!(Scalars(&db).select("key").await?, Some(Value::Integer(43)));
        Ok(())
    }
}
