use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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
pub struct EuroPerKilowattHour(#[serde_as(as = "serde_with::DisplayFromStr")] pub Decimal);
