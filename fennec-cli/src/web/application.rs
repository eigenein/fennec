use std::sync::{Arc, RwLock};

use crate::state::{HunterState, LoggerState};

/// TODO: this is actually more like "last result".
#[must_use]
#[derive(Clone)]
pub struct State {
    pub logger: Arc<RwLock<LoggerState>>,
    pub hunter: Arc<RwLock<HunterState>>,
}
