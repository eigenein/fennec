#[macro_use]
pub mod macros;

pub mod currency;
pub mod energy;
pub mod power;
pub mod price;
pub mod ratios;
pub mod time;
mod zero;

use std::ops::Mul;

pub use self::zero::Zero;

/// Generic quantity with dimensions of `P` over power, `T` over time, and `C` over cost.
#[must_use]
#[repr(transparent)]
#[derive(
    ::derive_more::Add,
    ::derive_more::AddAssign,
    ::derive_more::FromStr,
    ::derive_more::Neg,
    ::derive_more::Sub,
    ::derive_more::SubAssign,
    ::derive_more::Sum,
    ::musli::Decode,
    ::musli::Encode,
    ::serde::Deserialize,
    ::serde::Serialize,
    ::std::clone::Clone,
    ::std::cmp::Eq,
    ::std::cmp::Ord,
    ::std::cmp::PartialEq,
    ::std::cmp::PartialOrd,
    ::std::marker::Copy,
)]
#[musli(transparent)]
pub struct Quantity<V, const P: i8, const T: i8, const C: i8>(pub V);

#[rustfmt::skip]
macro_rules! format_quantity {
    ($name:ident, suffix: $suffix:literal, precision: $precision:literal) => {
        impl<V: ::std::fmt::Display> ::std::fmt::Display for $name<V> {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(formatter, "{0:.1$} {2}", self.0, $precision, $suffix)
            }
        }

        impl<V: ::std::fmt::Display> ::std::fmt::Debug for $name<V> {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(formatter, "{0:.1$}{2}", self.0, $precision, $suffix)
            }
        }

        impl<V: ::std::fmt::Display> ::maud::Render for $name<V> {
            fn render(&self) -> ::maud::Markup {
                ::maud::html! { (format!("{0:.1$}", self.0, $precision))
                (::maud::PreEscaped("&nbsp;"))
                ($suffix) }
            }
        }
    };
}

impl<V, const P: i8, const T: i8, const C: i8> Mul<V> for Quantity<V, P, T, C>
where
    V: Mul<Output = V>,
{
    type Output = Self;

    fn mul(self, rhs: V) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<const P: i8, const T: i8, const C: i8> Mul<Quantity<Self, P, T, C>> for f64 {
    type Output = Quantity<Self, P, T, C>;

    fn mul(self, rhs: Quantity<Self, P, T, C>) -> Self::Output {
        Quantity(self * rhs.0)
    }
}
