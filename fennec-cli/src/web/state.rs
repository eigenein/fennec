use chrono::{DateTime, Local};

use crate::prelude::*;

#[derive(Default)]
pub struct Application {
    pub logger: Option<Result<DateTime<Local>>>,
}
