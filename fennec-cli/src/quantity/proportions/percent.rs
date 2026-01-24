use std::{
    fmt::{Debug, Formatter},
    ops::Mul,
};

use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::quantity::proportions::BasisPoints;

#[derive(Copy, Clone, Eq, PartialEq, From, Serialize, Deserialize)]
pub struct Percent(u16);

impl Debug for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl Percent {
    pub const fn to_proportion(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Mul<Self> for Percent {
    type Output = BasisPoints;

    fn mul(self, rhs: Self) -> Self::Output {
        BasisPoints::from(self.0 * rhs.0)
    }
}
