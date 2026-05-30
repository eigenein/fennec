use std::fmt::{Debug, Display, Formatter};

use maud::{Markup, PreEscaped, Render, html};

use crate::quantity::Quantity;

/// Quantity display format.
pub trait Format {
    const SUFFIX: &str;
    const PRECISION: usize = 0;
}

impl<V, const S: i8, const P: i8, const T: i8, const C: i8> Display for Quantity<V, S, P, T, C>
where
    V: Display,
    Self: Format,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{0:.1$} {2}", self.0, Self::PRECISION, Self::SUFFIX)
    }
}

impl<V, const S: i8, const P: i8, const T: i8, const C: i8> Debug for Quantity<V, S, P, T, C>
where
    V: Display,
    Self: Format,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{0:.1$}{2}", self.0, Self::PRECISION, Self::SUFFIX)
    }
}

impl<V, const S: i8, const P: i8, const T: i8, const C: i8> Render for Quantity<V, S, P, T, C>
where
    V: Display,
    Self: Format,
{
    fn render(&self) -> Markup {
        html! {
            (format!("{0:.1$}", self.0, Self::PRECISION))
            (PreEscaped("&nbsp;"))
            (Self::SUFFIX)
        }
    }
}
