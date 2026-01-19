use tracing_subscriber::{fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_web::{MakeConsoleWriter, performance_layer};

pub fn init_tracing() {
    let format_layer = tracing_subscriber::fmt::layer().json().with_writer(MakeConsoleWriter);
    let performance_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry().with(format_layer).with(performance_layer).init();
}
