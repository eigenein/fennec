use std::sync::{Arc, RwLock};

use chrono::{DateTime, Local};
use chrono_humanize::HumanTime;
use maud::{Markup, html};

use crate::{
    prelude::*,
    state::{HunterState, LoggerState},
};

#[must_use]
#[derive(Clone)]
pub struct ApplicationState {
    pub logger: Arc<RwLock<SystemState<LoggerState>>>,
    pub hunter: Arc<RwLock<SystemState<HunterState>>>,
}

impl ApplicationState {
    pub fn error_message(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Err(error) = &self.logger.read().unwrap().result {
            parts.push(format!("Logger has failed: {error:#}."));
        }
        if let Err(error) = &self.hunter.read().unwrap().result {
            parts.push(format!("Hunter has failed: {error:#}."));
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    }
}

#[must_use]
pub struct SystemState<T> {
    pub last_run_at: DateTime<Local>,
    pub result: Result<T>,
}

impl<T> From<Result<T>> for SystemState<T> {
    fn from(result: Result<T>) -> Self {
        Self { last_run_at: Local::now(), result }
    }
}

impl<T> From<T> for SystemState<T> {
    fn from(state: T) -> Self {
        Self { last_run_at: Local::now(), result: Ok(state) }
    }
}

impl<T> SystemState<T> {
    /// Render short status for the navigation bar.
    pub fn status(&self) -> Markup {
        html! {
            @match self.result {
                Ok(_) => time datetime=(self.last_run_at.to_rfc3339()) {
                    (HumanTime::from(self.last_run_at))
                },
                Err(_) => {
                    "failed"
                },
            }
        }
    }
}
