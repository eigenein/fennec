use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    ops::{Div, Mul},
};

use chrono::TimeDelta;
use derive_more::{Add, FromStr, Neg, Sub};
use serde::{Deserialize, Serialize};

use crate::quantity::{energy::KilowattHours, power::kilowatts::Kilowatts};

#[derive(Copy, Clone, PartialOrd, PartialEq, FromStr, Add, Sub, Neg, Serialize, Deserialize)]
pub struct Watts(pub f64);

impl Watts {
    pub const fn zero() -> Self {
        Self(0.0)
    }
}

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

impl Eq for Watts {}

impl Ord for Watts {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Mul<TimeDelta> for Watts {
    type Output = KilowattHours;

    fn mul(self, time_delta: TimeDelta) -> Self::Output {
        Kilowatts::from(self) * time_delta
    }
}

impl Div<f64> for Watts {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}
