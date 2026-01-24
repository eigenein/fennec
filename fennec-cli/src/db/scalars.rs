use turso::{Connection, Value};

use crate::prelude::*;

#[must_use]
pub struct Scalars<'c>(pub &'c Connection);

impl Scalars<'_> {
    #[instrument(skip_all, fields(key = key))]
    pub async fn select<T: Selectable>(&self, key: &str) -> Result<T> {
        T::select_from(self, key).await
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

pub trait Selectable: Sized {
    async fn select_from(scalars: &Scalars<'_>, key: &str) -> Result<Self>;
}

impl Selectable for Value {
    async fn select_from(scalars: &Scalars<'_>, key: &str) -> Result<Self> {
        // language=sqlite
        const SQL: &str = "SELECT value FROM scalars WHERE key = ?1";
        match scalars.0.prepare_cached(SQL).await?.query_row((key,)).await {
            Ok(row) => Ok(row.get_value(0)?),
            Err(turso::Error::QueryReturnedNoRows) => Ok(Self::Null),
            Err(error) => Err(anyhow::format_err!(error)),
        }
    }
}

macro_rules! selectable {
    ($ty:ty, $member:path) => {
        impl Selectable for Option<$ty> {
            async fn select_from(scalars: &Scalars<'_>, key: &str) -> Result<Option<$ty>> {
                match Value::select_from(scalars, key).await? {
                    Value::Null => Ok(None),
                    $member(value) => Ok(Some(value)),
                    _ => bail!("`{key}` is not an `{}`", ::std::any::type_name::<$ty>()),
                }
            }
        }

        impl Selectable for $ty {
            async fn select_from(scalars: &Scalars<'_>, key: &str) -> Result<$ty> {
                Option::<$ty>::select_from(scalars, key)
                    .await?
                    .with_context(|| format!("no value stored for `{key}`"))
            }
        }
    };
}

selectable!(i64, Value::Integer);

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::db::Db;

    #[tokio::test]
    async fn scalars_ok() -> Result {
        let db = Db::connect(Path::new(":memory:")).await?;
        assert_eq!(Scalars(&db).select::<Option<i64>>("key").await?, None);
        Scalars(&db).upsert("key", Value::Integer(42)).await?;
        assert_eq!(Scalars(&db).select::<Option<i64>>("key").await?, Some(42));
        Scalars(&db).upsert("key", Value::Integer(43)).await?;
        assert_eq!(Scalars(&db).select::<Option<i64>>("key").await?, Some(43));
        Ok(())
    }
}
