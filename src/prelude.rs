#![allow(unused_imports)]

pub use anyhow::{Context, Error, bail, ensure};
pub use logfire::{debug, error, info, trace, warn};
pub use tracing::{Level, instrument};

pub type Result<T = (), E = Error> = anyhow::Result<T, E>;
