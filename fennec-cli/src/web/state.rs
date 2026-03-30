use chrono::{DateTime, Local};

use crate::{prelude::*, web::status::Status};

#[derive(Default)]
pub struct Application {
    pub logger: Option<Result<DateTime<Local>>>,
}

impl Application {
    pub const fn status(&self) -> Status {
        if matches!(self.logger, Some(Err(_))) {
            return Status::Error;
        }
        if self.logger.is_none() {
            return Status::Warning;
        }
        Status::Ok
    }
}
