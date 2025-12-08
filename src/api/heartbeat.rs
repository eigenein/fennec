use std::time::Duration;

use http::Uri;
use ureq::Agent;

use crate::prelude::*;

#[instrument(skip_all)]
pub fn send(uri: Uri) -> Result {
    info!(%uri, "Sending a heartbeat…");
    let agent: Agent =
        Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).build().into();
    agent.post(uri).send(())?;
    Ok(())
}
