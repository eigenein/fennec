use std::sync::{Arc, RwLock, RwLockReadGuard};

use chrono::{DateTime, Local};

use crate::state::{HunterState, LoggerState};

#[must_use]
#[derive(Clone)]
pub struct State {
    pub logger: Component<LoggerState>,
    pub hunter: Component<HunterState>,
}

#[must_use]
pub struct Component<T>(Arc<RwLock<ComponentInner<T>>>);

impl<T> Clone for Component<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Component<T> {
    pub fn now(state: T) -> Self {
        Self(Arc::new(RwLock::new(ComponentInner::now(state))))
    }

    pub fn get(&self) -> RwLockReadGuard<'_, ComponentInner<T>> {
        self.0.read().unwrap()
    }

    pub fn update(&self, state: T) {
        let mut lock = self.0.write().unwrap();
        lock.last_run_at = Local::now();
        lock.state = state;
    }
}

#[must_use]
pub struct ComponentInner<T> {
    pub last_run_at: DateTime<Local>,
    pub state: T,
}

impl<T> ComponentInner<T> {
    fn now(state: T) -> Self {
        Self { last_run_at: Local::now(), state }
    }
}
