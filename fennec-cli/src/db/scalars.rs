use turso::{Connection, Value};

use crate::{
    db::{key::Key, selectable::Selectable},
    prelude::*,
};

/// Collection of primitive key-value rows.
#[must_use]
pub struct Scalars<'c>(pub &'c Connection);

impl Scalars<'_> {
    #[instrument(skip_all, fields(key = ?key))]
    pub async fn select<T>(&self, key: Key) -> Result<Option<T>>
    where
        Option<T>: Selectable,
    {
        Option::<T>::select_from(self, key).await
    }

    #[instrument(skip_all, fields(key = ?key))]
    pub async fn upsert(&self, key: Key, value: impl Into<Value>) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO scalars (key, value) VALUES (?1, ?2)
            ON CONFLICT DO UPDATE SET value = ?2
        ";
        self.0.prepare_cached(SQL).await?.execute((key.as_str(), value)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::db::legacy_db::LegacyDb;

    #[tokio::test]
    async fn scalars_ok() -> Result {
        let db = LegacyDb::connect(Path::new(":memory:"), true).await?;
        assert_eq!(Scalars(&db).select::<i64>(Key::Test).await?, None);

        Scalars(&db).upsert(Key::Test, Value::Integer(42)).await?;
        assert_eq!(Scalars(&db).select::<i64>(Key::Test).await?, Some(42));

        Scalars(&db).upsert(Key::Test, Value::Integer(43)).await?;
        assert_eq!(Scalars(&db).select::<i64>(Key::Test).await?, Some(43));

        Ok(())
    }
}
