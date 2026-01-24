use turso::Value;

use crate::{
    db::{key::Key, scalars::Scalars},
    prelude::*,
    quantity::Quantity,
};

pub trait Selectable: Sized {
    async fn select_from(scalars: &Scalars<'_>, key: Key) -> Result<Self>;
}

impl Selectable for Value {
    async fn select_from(scalars: &Scalars<'_>, key: Key) -> Result<Self> {
        // language=sqlite
        const SQL: &str = "SELECT value FROM scalars WHERE key = ?1";
        match scalars.0.prepare_cached(SQL).await?.query_row((key.as_str(),)).await {
            Ok(row) => Ok(row.get_value(0)?),
            Err(turso::Error::QueryReturnedNoRows) => Ok(Self::Null),
            Err(error) => Err(anyhow::format_err!(error)),
        }
    }
}

macro_rules! scalar {
    ($member:path, $ty:ty) => {
        impl Selectable for Option<$ty> {
            async fn select_from(scalars: &Scalars<'_>, key: Key) -> Result<Option<$ty>> {
                match Value::select_from(scalars, key).await? {
                    Value::Null => Ok(None),
                    $member(value) => Ok(Some(value)),
                    _ => bail!("`{key:?}` is not an `{}`", ::std::any::type_name::<$ty>()),
                }
            }
        }
    };
}

scalar!(Value::Integer, i64);
scalar!(Value::Real, f64);

impl<const POWER: isize, const TIME: isize, const COST: isize> Selectable
    for Option<Quantity<POWER, TIME, COST>>
{
    async fn select_from(scalars: &Scalars<'_>, key: Key) -> Result<Self> {
        Ok(scalars.select_scalar::<f64>(key).await?.map(Quantity))
    }
}
