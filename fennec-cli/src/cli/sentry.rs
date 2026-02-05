use std::borrow::Cow;

use clap::{Parser, crate_name, crate_version};
use sentry::ClientInitGuard;

use crate::prelude::*;

#[derive(Parser)]
pub struct SentryArgs {
    #[clap(long = "sentry-dsn", env = "SENTRY_DSN")]
    sentry_dsn: Option<String>,
}

impl SentryArgs {
    pub fn init(&self) -> Option<ClientInitGuard> {
        self.sentry_dsn.as_deref().map_or_else(
            || {
                warn!("Sentry is not configured");
                None
            },
            |dsn| {
                let options = sentry::ClientOptions {
                    traces_sample_rate: 1.0,
                    sample_rate: 1.0,
                    send_default_pii: true,
                    attach_stacktrace: true,
                    in_app_include: vec![crate_name!()],
                    release: Some(Cow::Borrowed(crate_version!())),
                    ..Default::default()
                };
                Some(sentry::init((dsn, options)))
            },
        )
    }
}
