use std::path::Path;

use musli::{Decode, Encode, alloc::Global, mode::Binary, wire};

use crate::prelude::*;

pub trait File: Default + Encode<Binary> + for<'a> Decode<'a, Binary, Global> {
    const PATH: &str;

    #[instrument]
    async fn read() -> Result<Self> {
        let bytes = tokio::fs::read(Self::PATH).await.context("failed to read the file")?;
        wire::decode(bytes.as_slice()).context("failed to decode the file")
    }

    async fn read_or_default() -> Result<Self> {
        if Path::new(Self::PATH).exists() { Self::read().await } else { Ok(Self::default()) }
    }

    /// TODO: write to temporary file and rename for atomicity.
    #[instrument(skip_all, fields(path = Self::PATH))]
    async fn write(&self) -> Result {
        let bytes = wire::to_vec(self).context("failed to encode the energy profile")?;
        tokio::fs::write(Self::PATH, bytes.as_slice())
            .await
            .context("failed to write the energy profile")?;
        Ok(())
    }
}
