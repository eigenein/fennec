use std::fmt::{Debug, Formatter};

pub struct FormattedPercentage(pub f64);

impl Debug for FormattedPercentage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}
