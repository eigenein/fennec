use std::fmt::{Debug, Formatter};

use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, From, Serialize, Deserialize)]
pub struct Percent(u16);

impl Percent {
    pub const fn to_proportion(self) -> f64 {
        0.01 * self.0 as f64
    }
}

impl Debug for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}
