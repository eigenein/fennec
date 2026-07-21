use std::ops::{Add, Div, Mul, SubAssign};

use musli::{Decode, Encode};

use crate::quantity::{Zero, currency::Mills, energy::WattHours, price::KilowattHourPrice};

/// Generic bidirectional energy flow.
#[must_use]
#[expect(clippy::derive_partial_eq_without_eq)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Sum,
    derive_more::AddAssign,
    Encode,
    Decode,
)]
pub struct Flow<T> {
    /// Importing from grid or charging the battery.
    #[musli(Binary, name = 1)]
    pub import: T,

    /// Exporting to the grid or discharging the battery.
    #[musli(Binary, name = 2)]
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
    pub const fn reversed(self) -> Self
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

    pub fn total_throughput(self) -> <T as Add>::Output
    where
        T: Add,
    {
        self.import + self.export
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

impl Flow<f64> {
    /// Calculate the round-trip efficiency assuming each direction represents efficiency in that direction.
    pub const fn round_trip(self) -> f64 {
        self.import * self.export
    }
}

impl Flow<KilowattHourPrice> {
    /// Calculate the grid consumption loss minus production revenue.
    pub fn loss(self, energy: Flow<WattHours>) -> Mills {
        energy.import * self.import - energy.export * self.export
    }
}
