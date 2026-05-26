use std::sync::{Arc, RwLock};

use crate::cli::state;

/// TODO: this is actually more like "last result".
#[must_use]
#[derive(Clone)]
pub struct State {
    pub logger: Arc<RwLock<state::Logger>>,
    pub hunter: Arc<RwLock<state::Hunter>>,
}
