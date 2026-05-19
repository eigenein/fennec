use chrono::{DateTime, Local, TimeZone};
use musli::{Decoder, Encoder};

pub fn encode<E: Encoder>(timestamp: &DateTime<Local>, encoder: E) -> Result<(), E::Error> {
    encoder.encode(timestamp.timestamp_micros())
}

pub fn decode<'de, D: Decoder<'de>>(decoder: D) -> Result<DateTime<Local>, D::Error> {
    Ok(Local.timestamp_micros(decoder.decode()?).unwrap())
}
