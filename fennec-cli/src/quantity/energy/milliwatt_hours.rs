use derive_more::{From, Into};

/// Milliwatt-hours, 1 mWh = 0.001 Wh.
///
/// This awkward unit is used to track when the reported residual energy of a battery changes.
#[derive(Copy, Clone, From, Into)]
#[into(turso::Value)]
pub struct MilliwattHours(i64);
