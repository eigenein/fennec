use std::sync::{Arc, RwLock};

use chrono::{DateTime, Local};

use crate::state::{HunterState, LoggerState};

#[must_use]
#[derive(Clone)]
pub struct ApplicationState {
    pub logger: Arc<RwLock<SystemState<LoggerState>>>,
    pub hunter: Arc<RwLock<SystemState<HunterState>>>,
}

#[must_use]
pub struct SystemState<T> {
    pub last_run_at: DateTime<Local>,
    pub result: T,
}

impl<T> From<T> for SystemState<T> {
    fn from(result: T) -> Self {
        Self { last_run_at: Local::now(), result }
    }
}
