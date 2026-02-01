use turso::{Connection, Value};

use crate::{
    db::{legacy_key::LegacyKey, selectable::Selectable},
    prelude::*,
};

/// Collection of primitive key-value rows.
#[must_use]
pub struct LegacyScalars<'c>(pub &'c Connection);

impl LegacyScalars<'_> {
    #[instrument(skip_all, fields(key = ?key))]
    pub async fn select<T>(&self, key: LegacyKey) -> Result<Option<T>>
    where
        Option<T>: Selectable,
    {
        Option::<T>::select_from(self, key).await
    }

    #[instrument(skip_all, fields(key = ?key))]
    pub async fn upsert(&self, key: LegacyKey, value: impl Into<Value>) -> Result {
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
        assert_eq!(LegacyScalars(&db).select::<i64>(LegacyKey::Test).await?, None);

        LegacyScalars(&db).upsert(LegacyKey::Test, Value::Integer(42)).await?;
        assert_eq!(LegacyScalars(&db).select::<i64>(LegacyKey::Test).await?, Some(42));

        LegacyScalars(&db).upsert(LegacyKey::Test, Value::Integer(43)).await?;
        assert_eq!(LegacyScalars(&db).select::<i64>(LegacyKey::Test).await?, Some(43));

        Ok(())
    }
}
