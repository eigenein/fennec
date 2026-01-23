use std::fmt::{Debug, Formatter};

pub struct FormattedEfficiency(pub f64);

impl Debug for FormattedEfficiency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}
