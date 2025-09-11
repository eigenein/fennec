use anyhow::bail;
use serde::Deserialize;

/// Generic API response.
///
/// I first read the response into [`serde_json::Value`] in order to log it.
/// And only then, I do parse it.
#[derive(Deserialize)]
pub struct Response {
    /// Error code (when the result is not equal to zero, the request failed).
    #[serde(rename = "errno")]
    error_code: i32,

    #[serde(rename = "msg")]
    message: Option<String>,

    #[serde(rename = "result")]
    result: serde_json::Value,
}

impl From<Response> for crate::prelude::Result<serde_json::Value> {
    fn from(response: Response) -> Self {
        if response.error_code == 0 {
            Ok(response.result)
        } else if let Some(message) = response.message {
            bail!(
                r#"FoxESS Cloud error {error_code} ("{message}")"#,
                error_code = response.error_code,
            )
        } else {
            bail!("FoxESS Cloud error {error_code}", error_code = response.error_code)
        }
    }
}
