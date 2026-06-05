use axum::response::IntoResponse;
use http::{StatusCode, header};

use crate::prelude::*;

#[instrument(skip_all)]
pub async fn get() -> impl IntoResponse {
    debug!("check");
    (StatusCode::NO_CONTENT, [(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")])
}
