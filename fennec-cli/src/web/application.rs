use std::sync::{Arc, RwLock, RwLockReadGuard};

use crate::{
    ops::cache,
    state::{HunterState, LoggerState},
};

/// TODO: this is actually more like "last result".
#[must_use]
#[derive(Clone)]
pub struct State {
    pub logger: Component<LoggerState>,
    pub hunter: Component<HunterState>,
}

/// TODO: this is more like just a lock wrapper.
#[must_use]
pub struct Component<T>(Arc<RwLock<cache::Entry<T>>>);

impl<T> Clone for Component<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Component<T> {
    pub fn now(state: T) -> Self {
        Self(Arc::new(RwLock::new(cache::Entry::now(state))))
    }

    pub fn get(&self) -> RwLockReadGuard<'_, cache::Entry<T>> {
        self.0.read().unwrap()
    }

    pub fn update(&self, state: T) {
        *self.0.write().unwrap() = cache::Entry::now(state);
    }
}
