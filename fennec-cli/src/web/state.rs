use chrono::{DateTime, Local};
use chrono_humanize::HumanTime;
use maud::{Markup, html};

use crate::{prelude::*, web::status::Status};

#[derive(Default)]
pub struct ApplicationState {
    pub logger: SystemState<()>,
    pub solver: SystemState<()>,
}

impl ApplicationState {
    pub const fn status(&self) -> Status {
        if matches!(self.logger, SystemState::Err(_)) || matches!(self.solver, SystemState::Err(_))
        {
            return Status::Error;
        }
        if matches!(self.logger, SystemState::Pending)
            || matches!(self.solver, SystemState::Pending)
        {
            return Status::Warning;
        }
        Status::Ok
    }
}

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
                Self::Ok { last_run_at, .. } => span title=(last_run_at) {
                    (HumanTime::from(*last_run_at))
                },
                Self::Err(_) => "failed",
                Self::Pending => "pending",
            }
        }
    }
}
