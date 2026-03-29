use chrono::{DateTime, Local};
use derive_more::FromStr;
use tokio::time::sleep;

use crate::prelude::*;

#[derive(Clone, FromStr)]
pub struct CronSchedule(croner::Cron);

impl CronSchedule {
    pub fn start(self) -> Cron {
        Cron { schedule: self.0, last_timestamp: Local::now() }
    }
}

pub struct Cron {
    schedule: croner::Cron,
    last_timestamp: DateTime<Local>,
}

impl Cron {
    pub async fn wait_until_next(&mut self) -> Result {
        loop {
            let next = self.schedule.find_next_occurrence(&self.last_timestamp, false)?;
            self.last_timestamp = next;
            if let Ok(duration) = (next - Local::now()).to_std() {
                debug!(pattern = %self.schedule.pattern, ?next);
                sleep(duration).await;
                return Ok(());
            }
        }
    }
}
