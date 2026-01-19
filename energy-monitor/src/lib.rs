mod home_wizard;
mod result;
mod tracing;

use worker::{Env, ScheduleContext, ScheduledEvent};
use worker_macros::{event, send};

use crate::{result::Result, tracing::init_tracing};

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _context: ScheduleContext) {
    init_tracing();
    try_scheduled(env).await.unwrap();
}

async fn try_scheduled(env: Env) -> Result {
    let p1_service = env.service("p1Service")?;
    Ok(())
}
