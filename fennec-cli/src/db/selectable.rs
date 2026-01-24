use turso::Value;

use crate::{db::scalars::Scalars, prelude::*};

pub trait Selectable: Sized {
    async fn select_from(scalars: &Scalars<'_>, key: &str) -> crate::prelude::Result<Self>;
}

impl Selectable for Value {
    async fn select_from(scalars: &Scalars<'_>, key: &str) -> crate::prelude::Result<Self> {
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
