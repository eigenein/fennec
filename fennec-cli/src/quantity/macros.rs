macro_rules! quantity {
    ($name:ident, $container:tt, $unit:literal) => {
        #[repr(transparent)]
        #[derive(
            ::derive_more::Add,
            ::derive_more::AddAssign,
            ::derive_more::FromStr,
            ::derive_more::Sub,
            ::derive_more::SubAssign,
            ::derive_more::Sum,
            ::serde::Deserialize,
            ::serde::Serialize,
            ::std::clone::Clone,
            ::std::marker::Copy,
        )]
        pub struct $name(pub $container);

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, formatter)?;
                write!(formatter, " {}", $unit)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Debug::fmt(&self.0, formatter)?;
                write!(formatter, "{}", $unit)
            }
        }

        impl ::std::cmp::PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl ::std::cmp::Eq for $name {}

        impl $name {
            pub const fn zero() -> Self {
                Self(0 as $container)
            }
        }

        operations!($name, $container);
    };
}

/// Implement ordering for the quantity.
macro_rules! operations {
    ($name:path,f64) => {
        derive_neg!($name);
        ordered_float!($name);

        impl Mul<f64> for $name {
            type Output = Self;

            fn mul(self, rhs: f64) -> Self::Output {
                Self(self.0 * rhs)
            }
        }

        impl Div<Self> for $name {
            type Output = f64;

            fn div(self, rhs: Self) -> Self::Output {
                self.0 / rhs.0
            }
        }

        impl Div<f64> for $name {
            type Output = Self;

            fn div(self, rhs: f64) -> Self::Output {
                Self(self.0 / rhs)
            }
        }
    };
    ($name:path,u16) => {
        derive_ord!($name);
        derive_partial_eq!($name);
    };
    ($name:path,i64) => {
        derive_neg!($name);
        derive_ord!($name);
        derive_partial_eq!($name);
    };
}

/// Derive ordering and equality via [`::ordered_float::OrderedFloat`].
macro_rules! ordered_float {
    ($name:path) => {
        impl ::std::cmp::Ord for $name {
            fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                ::ordered_float::OrderedFloat(self.0).cmp(&::ordered_float::OrderedFloat(other.0))
            }
        }

        impl ::std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                ::ordered_float::OrderedFloat(self.0).eq(&::ordered_float::OrderedFloat(other.0))
            }
        }
    };
}

/// Derive [`::std::ops::Neg`] from the container type.
macro_rules! derive_neg {
    ($name:path) => {
        impl ::std::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self(-self.0)
            }
        }
    };
}

macro_rules! derive_ord {
    ($name:path) => {
        impl ::std::cmp::Ord for $name {
            fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }
    };
}

macro_rules! derive_partial_eq {
    ($name:path) => {
        impl ::std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.eq(&other.0)
            }
        }
    };
}
