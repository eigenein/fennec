use chrono::Local;
use serde::{Serialize, Serializer};

/// Serialize the timestamp as timeseries-suitable timestamp.
///
/// PS. Yeah, it does not make sense but MongoDB will not accept the default Chrono serialization.
pub fn serialize_timestamp<S: Serializer>(
    timestamp: &chrono::DateTime<Local>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    bson::DateTime::from(*timestamp).serialize(serializer)
}
