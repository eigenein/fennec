use std::path::Path;

use musli::{Decode, Encode, alloc::Global, mode::Binary, wire};

use crate::prelude::*;

pub trait File: Default + Encode<Binary> + for<'a> Decode<'a, Binary, Global> {
    const PATH: &str;

    #[instrument]
    async fn read() -> Result<Option<Self>> {
        let path = Path::new(Self::PATH);
        if path.exists() {
            let bytes = tokio::fs::read(path).await.context("failed to read the file")?;
            wire::decode(bytes.as_slice()).context("failed to decode the file")
        } else {
            Ok(None)
        }
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

pub mod chrono {
    use chrono::{DateTime, Local, TimeZone};
    use musli::{Decoder, Encoder};

    pub fn encode<E: Encoder>(timestamp: &DateTime<Local>, encoder: E) -> Result<(), E::Error> {
        encoder.encode(timestamp.timestamp_micros())
    }

    pub fn decode<'de, D: Decoder<'de>>(decoder: D) -> Result<DateTime<Local>, D::Error> {
        Ok(Local.timestamp_micros(decoder.decode()?).unwrap())
    }
}
