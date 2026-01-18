use worker::*;

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, _env: Env, _context: ScheduleContext) {}
