use crate::{
    db::{key::Key, scalars::Scalars},
    prelude::*,
};

pub trait Compound: Sized {
    async fn select_from(scalars: &Scalars<'_>) -> Result<Self>;
}

#[derive(Copy, Clone)]
pub struct SchemaVersion(pub i64);

impl Compound for SchemaVersion {
    async fn select_from(scalars: &Scalars<'_>) -> Result<Self> {
        Ok(Self(scalars.select_scalar::<i64>(Key::SchemaVersion).await?.unwrap_or_default()))
    }
}
