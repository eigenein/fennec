use time::{UtcDateTime, format_description::well_known::Iso8601};
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};
use worker::Date;

pub struct WorkerFormatTime;

impl FormatTime for WorkerFormatTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> std::fmt::Result {
        let timestamp_nanos = i128::from(Date::now().as_millis()) * 1_000_000;
        let timestamp = UtcDateTime::from_unix_timestamp_nanos(timestamp_nanos)
            .expect("current timestamp must be valid");
        writer.write_str(
            &timestamp.format(&Iso8601::DEFAULT).expect("current timestamp must be formattable"),
        )
    }
}
