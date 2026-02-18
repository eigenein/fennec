use std::fmt::{Debug, Display, Formatter};

use crate::quantity::power::kilowatts::Kilowatts;

#[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Watts(f64);

impl From<Kilowatts> for Watts {
    fn from(kilowatts: Kilowatts) -> Self {
        Self(kilowatts.0 * 1000.0)
    }
}

impl Debug for Watts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}W", self.0)
    }
}

impl Display for Watts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0} W", self.0)
    }
}
