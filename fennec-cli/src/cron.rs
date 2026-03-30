use chrono::{DateTime, Local};
use derive_more::FromStr;
use tokio::time::sleep;

use crate::prelude::*;

#[derive(Clone, FromStr)]
pub struct CronSchedule(croner::Cron);

impl CronSchedule {
    pub fn start(self) -> Cron {
        Cron { schedule: self.0, pointer: Local::now() }
    }
}

pub struct Cron {
    schedule: croner::Cron,
    pointer: DateTime<Local>,
}

impl Cron {
    pub async fn wait_until_next(&mut self) -> Result {
        loop {
            self.pointer = self.schedule.find_next_occurrence(&self.pointer, false)?;
            if let Ok(duration) = (self.pointer - Local::now()).to_std() {
                debug!(pattern = %self.schedule.pattern, next_timestamp = ?self.pointer, "sleeping…");
                sleep(duration).await;
                return Ok(());
            }
        }
    }
}
