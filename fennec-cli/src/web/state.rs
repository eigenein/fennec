use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local};
use chrono_humanize::HumanTime;
use maud::{Markup, html};

use crate::{prelude::*, state::SolverState};

#[must_use]
#[derive(Clone)]
pub struct ApplicationState {
    pub logger: Arc<Mutex<SystemState<()>>>,
    pub solver: Arc<Mutex<SystemState<SolverState>>>,
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self {
            logger: Arc::new(Mutex::new(SystemState::default())),
            solver: Arc::new(Mutex::new(SystemState::default())),
        }
    }
}

#[must_use]
#[derive(Default)]
pub enum SystemState<T> {
    Ok {
        last_run_at: DateTime<Local>,
        inner: T,
    },

    #[default]
    Pending,

    Err(Error),
}

impl<T> SystemState<T> {
    pub fn ok(inner: T) -> Self {
        Self::Ok { last_run_at: Local::now(), inner }
    }

    /// Render short status for the navigation bar.
    pub fn status(&self) -> Markup {
        html! {
            @match self {
                Self::Ok { last_run_at, .. } => time datetime=(last_run_at.to_rfc3339()) {
                    (HumanTime::from(*last_run_at))
                },
                Self::Err(_) => "failed",
                Self::Pending => "pending",
            }
        }
    }
}
