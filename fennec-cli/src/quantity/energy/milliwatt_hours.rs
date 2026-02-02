use std::fmt::{Debug, Formatter};

use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

/// Milliwatt-hours, 1 mWh = 0.001 Wh.
///
/// This awkward unit is used to track when the reported residual energy of a battery changes.
#[derive(Copy, Clone, Eq, PartialEq, From, Into, Serialize, Deserialize)]
pub struct MilliwattHours(i64);

impl Debug for MilliwattHours {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} mWh", self.0)
    }
}

impl From<MilliwattHours> for f64 {
    #[expect(clippy::cast_precision_loss)]
    fn from(value: MilliwattHours) -> Self {
        value.0 as Self
    }
}
