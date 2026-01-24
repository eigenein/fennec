use turso::{Connection, Value};

use crate::{
    db::{key::Key, primitive::Primitive},
    prelude::*,
};

#[must_use]
pub struct Scalars<'c>(pub &'c Connection);

impl Scalars<'_> {
    #[instrument(skip_all, fields(key = ?key))]
    pub async fn select_primitive<T: Primitive>(&self, key: Key) -> Result<T> {
        T::select_from(self, key).await
    }

    #[instrument(skip_all, fields(key = ?key))]
    pub async fn upsert(&self, key: Key, value: Value) -> Result {
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
    use crate::db::Db;

    #[tokio::test]
    async fn scalars_ok() -> Result {
        let db = Db::connect(Path::new(":memory:")).await?;
        assert_eq!(Scalars(&db).select_primitive::<Option<i64>>(Key::SchemaVersion).await?, None);

        Scalars(&db).upsert(Key::SchemaVersion, Value::Integer(42)).await?;
        assert_eq!(
            Scalars(&db).select_primitive::<Option<i64>>(Key::SchemaVersion).await?,
            Some(42)
        );

        Scalars(&db).upsert(Key::SchemaVersion, Value::Integer(43)).await?;
        assert_eq!(
            Scalars(&db).select_primitive::<Option<i64>>(Key::SchemaVersion).await?,
            Some(43)
        );
        Ok(())
    }
}
