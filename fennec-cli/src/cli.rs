pub mod battery;
mod burrow;
mod db;
mod estimation;
mod foxess;
mod hunt;
mod log;

use std::ops::RangeInclusive;

use chrono::{DateTime, Local, TimeDelta, Timelike};
use clap::{Parser, Subcommand};
use http::Uri;

use crate::{
    api::home_assistant,
    cli::{burrow::BurrowArgs, hunt::HuntArgs, log::LogArgs},
};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true)]
#[must_use]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Main command: fetch the prices, optimize the schedule, and push it to the cloud.
    #[clap(name = "hunt")]
    Hunt(Box<HuntArgs>),

    /// Log meter and battery measurements.
    #[clap(name = "log")]
    Log(Box<LogArgs>),

    /// Development tools.
    #[clap(name = "burrow")]
    Burrow(Box<BurrowArgs>),
}

#[deprecated]
#[derive(Parser)]
pub struct HomeAssistantArgs {
    #[clap(flatten)]
    pub connection: HomeAssistantConnectionArgs,

    #[clap(long = "home-assistant-entity-id", env = "HOME_ASSISTANT_ENTITY_ID")]
    pub entity_id: String,

    /// TODO: use `humantime`.
    #[clap(
        long = "home-assistant-history-days",
        default_value = "14",
        env = "HOME_ASSISTANT_HISTORY_DAYS"
    )]
    pub n_history_days: i64,
}

impl HomeAssistantArgs {
    pub fn history_period(&self) -> RangeInclusive<DateTime<Local>> {
        let now = Local::now();
        let now = now.with_nanosecond(0).unwrap_or(now);
        (now - TimeDelta::days(self.n_history_days))..=now
    }
}

#[deprecated]
#[derive(Parser)]
pub struct HomeAssistantConnectionArgs {
    /// Home Assistant API access token.
    #[clap(long = "home-assistant-access-token", env = "HOME_ASSISTANT_ACCESS_TOKEN")]
    pub access_token: String,

    /// Home Assistant API base URL. For example: `http://localhost:8123/api`.
    #[clap(long = "home-assistant-api-base-url", env = "HOME_ASSISTANT_API_BASE_URL")]
    pub base_url: Uri,
}

impl HomeAssistantConnectionArgs {
    pub fn new_client(&self) -> home_assistant::Api {
        home_assistant::Api::new(&self.access_token, self.base_url.clone())
    }
}
