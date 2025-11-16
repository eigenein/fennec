mod models;
mod response;
mod schedule;

use std::time::Duration;

use chrono::Utc;
use models::DeviceDetails;
use reqwest::{
    Client,
    Method,
    header::{HeaderMap, HeaderValue},
};
use response::Response;
use serde::{Serialize, de::DeserializeOwned};

pub use self::schedule::{TimeSlot, TimeSlotSequence, WorkingMode};
use self::{
    models::{DeviceRealTimeData, DeviceVariables},
    schedule::Schedule,
};
use crate::prelude::*;

pub struct Api {
    client: Client,
    api_key: String,
}

impl Api {
    pub fn try_new(api_key: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Timezone", HeaderValue::from_static("Europe/Amsterdam"));
        headers.insert("Lang", HeaderValue::from_static("en"));
        headers.insert("Token", HeaderValue::from_str(&api_key)?);
        let client = Client::builder()
            .user_agent("fennec")
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { client, api_key })
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn get_device_details(&self, serial_number: &str) -> Result<DeviceDetails> {
        info!("Fetching…");

        #[derive(Serialize)]
        struct GetDeviceDetailsRequest<'a> {
            #[serde(rename = "sn")]
            serial_number: &'a str,
        }

        self.call(Method::GET, "op/v0/device/detail", GetDeviceDetailsRequest { serial_number }, ())
            .await
            .context("failed to request the device details")
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn get_device_variables(&self, serial_number: &str) -> Result<DeviceVariables> {
        let variables = self
            .get_devices_variables_raw(&[serial_number])
            .await?
            .pop()
            .with_context(|| format!("no device `{serial_number}` in the response"))?
            .variables
            .into_iter()
            .map(|variable| (variable.name, variable.value))
            .collect::<serde_json::Map<_, _>>();
        serde_json::from_value(serde_json::Value::Object(variables))
            .context("failed to deserialize the device variables")
    }

    #[instrument(
        skip_all,
        level = Level::DEBUG,
        fields(serial_numbers = ?serial_numbers),
    )]
    pub async fn get_devices_variables_raw(
        &self,
        serial_numbers: &[&str],
    ) -> Result<Vec<DeviceRealTimeData>> {
        #[derive(Serialize)]
        struct GetDeviceRealTimeDataRequest<'a> {
            #[serde(rename = "sns")]
            serial_numbers: &'a [&'a str],
        }

        self.call(
            Method::POST,
            "op/v1/device/real/query",
            (),
            &GetDeviceRealTimeDataRequest { serial_numbers },
        )
        .await
        .context("failed to get the devices variables")
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn get_schedule(&self, serial_number: &str) -> Result<Schedule> {
        #[derive(Serialize)]
        struct GetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,
        }

        self.call(
            Method::POST,
            "op/v1/device/scheduler/get",
            (),
            &GetScheduleRequest { serial_number },
        )
        .await
        .context("failed to get the schedule")
    }

    #[instrument(skip_all, fields(serial_number = serial_number))]
    pub async fn set_schedule(&self, serial_number: &str, groups: &[TimeSlot]) -> Result {
        info!(n_groups = groups.len(), "Setting…");

        #[derive(Serialize)]
        struct SetScheduleRequest<'a> {
            #[serde(rename = "deviceSN")]
            serial_number: &'a str,

            #[serde(rename = "groups")]
            groups: &'a [TimeSlot],
        }

        self.call(
            Method::POST,
            "op/v1/device/scheduler/enable",
            (),
            SetScheduleRequest { serial_number, groups },
        )
        .await
    }

    #[instrument(skip_all, level = Level::DEBUG, fields(path = path))]
    async fn call<Q: Serialize, B: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Q,
        body: B,
    ) -> Result<R> {
        let (timestamp, signature) = self.build_signature(path);
        let response = Result::<serde_json::Value>::from(
            self.client
                .request(method, format!("https://www.foxesscloud.com/{path}"))
                .header("Timestamp", timestamp)
                .header("Signature", signature)
                .query(&query)
                .json(&body)
                .send()
                .await
                .with_context(|| format!("failed to call `{path}`"))?
                .error_for_status()
                .with_context(|| format!("`{path}` failed"))?
                .json::<Response>()
                .await
                .with_context(|| format!("failed to deserialize `{path}` response JSON"))?,
        )?;
        debug!(?response, "Call succeeded");
        serde_json::from_value(response)
            .with_context(|| format!("failed to deserialize `{path}` response structure"))
    }

    /// WHOA-MEGA-SUPER-SECURE AUTHENTICATION!
    fn build_signature(&self, path: &str) -> (String, String) {
        let timestamp = Utc::now().timestamp_millis().to_string();

        // DearFoxESS API developers…
        // WHAT THE FUCK is with `\r\n` being RAW LITERALS?! You okay guys?!
        let digest =
            md5::compute(format!(r"/{path}\r\n{0}\r\n{timestamp}", self.api_key).as_bytes());

        let signature = format!("{digest:x}");
        (timestamp, signature)
    }
}
