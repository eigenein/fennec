use rust_decimal::Decimal;

use crate::units::Quantity;

/// Euro per kilowatt-hour.
pub type KilowattHourRate = Quantity<Decimal, 1, 0, 1, -1>;
