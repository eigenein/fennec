use serde::{Deserialize, Serialize};

use crate::quantity::power::Kilowatts;

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Statistics {
    pub household: Household,
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct Household {
    pub hourly_stand_by_power: [Option<Kilowatts>; 24],
}
