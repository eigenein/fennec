use anyhow::Context;
use rust_decimal::{Decimal, dec, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::prelude::*;

#[derive(
    Copy,
    Clone,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Neg,
    derive_more::Sub,
)]
pub struct Kilowatts(pub Decimal);

impl Kilowatts {
    pub fn from_watts_u32(watts: u32) -> Self {
        Self(Decimal::from(watts) * dec!(0.001))
    }
}

#[derive(Copy, Clone)]
pub struct Watts(pub Decimal);

impl From<Kilowatts> for Watts {
    fn from(kilowatts: Kilowatts) -> Self {
        Self(kilowatts.0 * dec!(1000))
    }
}

impl TryFrom<Watts> for u32 {
    type Error = Error;

    fn try_from(watts: Watts) -> Result<Self> {
        watts.0.to_u32().with_context(|| format!("could not convert {} watts to `u32`", watts.0))
    }
}

#[derive(
    Copy,
    Clone,
    Deserialize,
    derive_more::Display,
    derive_more::FromStr,
    derive_more::Sum,
    derive_more::Add,
    derive_more::Sub,
)]
pub struct KilowattHours(pub Decimal);

#[derive(
    Copy, Clone, PartialOrd, Ord, PartialEq, Eq, derive_more::Display, derive_more::FromStr,
)]
pub struct Euro(pub Decimal);

#[serde_as]
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::FromStr,
    derive_more::Sub,
    Serialize,
    Deserialize,
)]
pub struct EuroPerKilowattHour(#[serde_as(as = "DisplayFromStr")] pub Decimal);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watts_from_kilowatts_ok() {
        assert_eq!(Watts::from(Kilowatts(dec!(0.8))).0, dec!(800));
    }

    #[test]
    fn test_watts_to_u32_ok() -> Result {
        assert_eq!(u32::try_from(Watts(dec!(123.34)))?, 123);
        Ok(())
    }
}
