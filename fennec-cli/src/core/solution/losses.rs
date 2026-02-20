use derive_more::Add;

use crate::quantity::{Zero, currency::Mills};

#[must_use]
#[derive(Copy, Clone, Add)]
pub struct Losses {
    /// Cumulative loss to the grid till the end of the forecast period.
    pub grid: Mills,

    /// Cumulative loss to the battery health till the end of the forecast period.
    pub battery: Mills,
}

impl Zero for Losses {
    const ZERO: Self = Self { grid: Mills::ZERO, battery: Mills::ZERO };
}

impl Losses {
    pub fn total(self) -> Mills {
        self.grid + self.battery
    }
}
