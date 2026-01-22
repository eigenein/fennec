mod response;
mod schedule;

use std::time::Duration;

use chrono::Utc;
use serde::{Serialize, de::DeserializeOwned};
use ureq::Agent;

use self::schedule::Schedule;
pub use self::schedule::{TimeSlot, TimeSlotSequence, WorkingMode};
use crate::{api::foxess::response::Response, prelude::*};

pub struct Api {
    client: Agent,
    api_key: String,
}

impl Api {
    pub fn new(api_key: String) -> Self {
        let client = Agent::config_builder()
            .user_agent("fennec")
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into();
        Self { client, api_key }
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub fn get_schedule(&self, serial_number: &str) -> Result<Schedule> {
        #[derive(Serialize)]
        struct GetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,
        }

        self.post("op/v1/device/scheduler/get", &GetScheduleRequest { serial_number })
            .context("failed to get the schedule")
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub fn set_schedule(&self, serial_number: &str, groups: &[TimeSlot]) -> Result {
        info!(n_groups = groups.len(), "Setting…");

        #[derive(Serialize)]
        struct SetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,

            #[serde(rename = "groups")]
            groups: &'a [TimeSlot],
        }

        self.post("op/v1/device/scheduler/enable", SetScheduleRequest { serial_number, groups })
    }

    #[instrument(skip_all, level = Level::DEBUG, fields(path = path))]
    fn post<B, R>(&self, path: &str, body: B) -> Result<R>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let (timestamp, signature) = self.build_signature(path);
        self.client
            .post(format!("https://www.foxesscloud.com/{path}"))
            .header("Timestamp", timestamp)
            .header("Signature", signature)
            .header("Timezone", "Europe/Amsterdam")
            .header("Lang", "en")
            .header("Token", &self.api_key)
            .send_json(body)
            .with_context(|| format!("failed to call `{path}`"))?
            .body_mut()
            .read_json::<Response<R>>()
            .with_context(|| format!("failed to deserialize `{path}` response JSON"))?
            .into()
    }

    /// WHOA-MEGA-SUPER-SECURE AUTHENTICATION!
    fn build_signature(&self, path: &str) -> (String, String) {
        let timestamp = Utc::now().timestamp_millis().to_string();

        // Dear FoxESS API developers… what were you smoking while making `\r\n` RAW LITERALS?!
        let digest =
            md5::compute(format!(r"/{path}\r\n{0}\r\n{timestamp}", self.api_key).as_bytes());

        let signature = format!("{digest:x}");
        (timestamp, signature)
    }
}
