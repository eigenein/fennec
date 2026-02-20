use std::ops::{Div, Mul, SubAssign};

use derive_more::{Add, AddAssign, Sub};

use crate::quantity::Zero;

/// Generic bidirectional energy flow.
#[must_use]
#[derive(Copy, Clone, Add, Sub, AddAssign)]
pub struct Flow<T> {
    /// Importing from grid or charging the battery.
    pub import: T,

    /// Exporting to the grid or discharging the battery.
    pub export: T,
}

impl<T: Zero> Zero for Flow<T> {
    const ZERO: Self = Self { import: T::ZERO, export: T::ZERO };
}

impl<T> Flow<T> {
    /// Get the reversed flow where the import becomes export and vice versa.
    ///
    /// This is used to off-load unserved battery flow onto the grid:
    ///
    /// - Unserved charge becomes grid export
    /// - Unserved discharge becomes grid import
    pub const fn reversed(&self) -> Self
    where
        T: Copy,
    {
        Self { import: self.export, export: self.import }
    }

    pub fn normalized(mut self) -> Self
    where
        T: Zero + PartialOrd + SubAssign,
    {
        if self.import < T::ZERO {
            self.export -= self.import;
            self.import = T::ZERO;
        }
        if self.export < T::ZERO {
            self.import -= self.export;
            self.export = T::ZERO;
        }
        self
    }
}

impl<T: Mul<Rhs>, Rhs: Copy> Mul<Rhs> for Flow<T> {
    type Output = Flow<<T as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Flow { import: self.import * rhs, export: self.export * rhs }
    }
}

impl<T: Div<Rhs>, Rhs: Copy> Div<Rhs> for Flow<T> {
    type Output = Flow<<T as Div<Rhs>>::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        Flow { import: self.import / rhs, export: self.export / rhs }
    }
}
