use crate::quantity::energy::KilowattHours;

/// Decawatt-hours, 1 daWh = 10 Wh.
#[derive(Copy, Clone, derive_more::From)]
pub struct DecawattHours(u16);

impl From<DecawattHours> for KilowattHours {
    fn from(value: DecawattHours) -> Self {
        Self(0.01 * f64::from(value.0))
    }
}
