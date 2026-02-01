use turso::Value;

use crate::{
    db::{legacy_key::LegacyKey, scalars::LegacyScalars},
    prelude::*,
    quantity::energy::MilliwattHours,
};

pub trait Selectable: Sized {
    async fn select_from(scalars: &LegacyScalars<'_>, key: LegacyKey) -> Result<Self>;
}

impl Selectable for Value {
    async fn select_from(scalars: &LegacyScalars<'_>, key: LegacyKey) -> Result<Self> {
        // language=sqlite
        const SQL: &str = "SELECT value FROM scalars WHERE key = ?1";
        match scalars.0.prepare_cached(SQL).await?.query_row((key.as_str(),)).await {
            Ok(row) => Ok(row.get_value(0)?),
            Err(turso::Error::QueryReturnedNoRows) => Ok(Self::Null),
            Err(error) => Err(anyhow::format_err!(error)),
        }
    }
}

macro_rules! selectable {
    ($member:path, $ty:ty) => {
        impl Selectable for Option<$ty> {
            async fn select_from(
                scalars: &LegacyScalars<'_>,
                key: LegacyKey,
            ) -> Result<Option<$ty>> {
                match Value::select_from(scalars, key).await? {
                    Value::Null => Ok(None),
                    $member(value) => Ok(Some(value)),
                    _ => bail!("`{key:?}` is not an `{}`", ::std::any::type_name::<$ty>()),
                }
            }
        }
    };
}

selectable!(Value::Integer, i64);
selectable!(Value::Real, f64);

macro_rules! selectable_into {
    ($inner:ty, $outer:ty) => {
        impl Selectable for Option<$outer> {
            async fn select_from(scalars: &LegacyScalars<'_>, key: LegacyKey) -> Result<Self> {
                Ok(scalars.select::<$inner>(key).await?.map(Into::into))
            }
        }
    };
}

selectable_into!(i64, MilliwattHours);
