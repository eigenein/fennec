mod response;
mod schedule;
mod working_mode;

use std::time::Duration;

use chrono::Utc;
use http::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};
use serde_with::serde_as;

use self::schedule::Schedule;
pub use self::schedule::{Group, Groups};
use crate::{api::foxess::response::Response, prelude::*};

pub struct Api {
    client: Client,
    api_key: String,
}

impl Api {
    pub fn new(api_key: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.append("Timezone", HeaderValue::from_static("Europe/Amsterdam"));
        headers.append("Lang", HeaderValue::from_static("en"));
        headers.append("Token", HeaderValue::from_str(&api_key)?);
        let client = Client::builder()
            .user_agent("fennec")
            .timeout(Duration::from_secs(15))
            .default_headers(headers)
            .build()?;
        Ok(Self { client, api_key })
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn get_schedule(&self, serial_number: &str) -> Result<Schedule> {
        #[derive(Serialize)]
        struct GetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,
        }

        info!("getting…");
        self.post("op/v2/device/scheduler/get", &GetScheduleRequest { serial_number })
            .await
            .context("failed to get the schedule")
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn set_schedule(&self, serial_number: &str, groups: &[Group]) -> Result {
        #[serde_as]
        #[derive(Serialize)]
        struct SetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,

            /// Whether to restore the extra parameters like limits and cut-offs to the system defaults.
            #[serde(rename = "isDefault")]
            #[serde_as(as = "serde_with::BoolFromInt")]
            is_default_extra: bool,

            #[serde(rename = "groups")]
            groups: &'a [Group],
        }

        info!(n_groups = groups.len(), "setting…");
        self.post(
            "op/v2/device/scheduler/enable",
            SetScheduleRequest { serial_number, is_default_extra: true, groups },
        )
        .await
    }

    #[instrument(skip_all, level = Level::DEBUG, fields(path = path))]
    async fn post<B, R>(&self, path: &str, body: B) -> Result<R>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let (timestamp, signature) = self.build_signature(path);
        let response = self
            .client
            .post(format!("https://www.foxesscloud.com/{path}"))
            .header("Timestamp", timestamp)
            .header("Signature", signature)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("failed to call `{path}`"))?;
        let text = response.text().await?;
        debug!(text, "fetched the response");
        serde_json::from_str::<Response<R>>(&text)
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
