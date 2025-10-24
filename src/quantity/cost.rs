use std::fmt::{Display, Formatter};

use crate::quantity::Quantity;

pub type Cost = Quantity<f64, 0, 0, 1>;

impl Cost {
    pub const ONE_CENT: Self = Self(0.01);

    /// Round the cost to [Pips][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Percentage_in_point
    pub fn round_to_pips(self) -> Self {
        Self((self.0 * 10000.0).round() / 10000.0)
    }
}

impl Display for Cost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:+.2} €", self.0)
    }
}

impl From<Cost> for opentelemetry::Value {
    fn from(value: Cost) -> Self {
        format!("{:.2}€", value.0).into()
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn test_round_to_pips() {
        assert_abs_diff_eq!(Cost::from(0.0015).round_to_pips().0, 0.0015);
        assert_abs_diff_eq!(Cost::from(0.00016).round_to_pips().0, 0.0002);
    }
}
