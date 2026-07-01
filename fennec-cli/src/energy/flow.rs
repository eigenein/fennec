use std::ops::{Div, Mul, SubAssign};

use derive_more::{Add, AddAssign, Sub};
use musli::{Decode, Encode};

use crate::quantity::{
    Zero,
    currency::Mills,
    energy::WattHours,
    power::Watts,
    price::KilowattHourPrice,
};

/// Generic bidirectional energy flow.
#[must_use]
#[expect(clippy::derive_partial_eq_without_eq)]
#[derive(Copy, Clone, Debug, PartialEq, Add, Sub, AddAssign, Encode, Decode)]
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

    /// TODO: test and verify the invariant.
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

    /// The net import stays invariant under rebalancing.
    #[cfg(test)]
    pub fn invariant(self) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        self.import - self.export
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

impl Flow<Watts> {
    /// Bring down to zero any value under the threshold.
    ///
    /// Note that this does *not* preserve the invariant so use the lowest possible threshold.
    pub fn denoised(mut self, threshold: Watts) -> Self {
        if self.import < threshold {
            self.import = Watts::ZERO;
        }
        if self.export < threshold {
            self.export = Watts::ZERO;
        }
        self
    }
}
